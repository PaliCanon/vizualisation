// map.palicanon.app / map.dev.palicanon.app → vizualisation on Cloud Run.
//
// From search-front/infra/worker/proxy.js; the code is unchanged, only this
// comment names the hosts. The dev password (HTTP Basic, the service's
// gate) is relayed like any header, and so is the 401 that asks for it: the
// Worker adds the origin header and nothing else.
//
// Cloudflare is the front door (WAF, rate limits per IP); this relays the
// request as is — method, headers, body — to ORIGIN, the Cloud Run URL, and
// adds the one header the service demands: `x-origin-auth`, the secret
// `ORIGIN_AUTH` of this Worker. The service answers 403 to any request
// without it, so the `*.run.app` URL is no way round Cloudflare. The secret
// is set through the Cloudflare API (`infra/scripts/origin-secret push`),
// never by tofu; if it is missing (an apply that re-uploaded the script can
// drop it) the Worker answers 500 and calls nothing, so a dropped secret
// shows at once instead of as a storm of 403s from the service.
//
// Nothing is cached here: the pages are server-rendered per request.
//
// Redirects are relayed untouched (`redirect: "manual"`), except one thing:
// a `Location` on the *.run.app origin is rewritten to the public origin (the
// binding `PUBLIC_ORIGIN`, https like `ORIGIN`, missing → 500 and no fetch;
// never the request's own Host), or a
// redirect would send the visitor round the front door. "On the origin" is
// decided on the parsed URL (`.origin` equal), never on a string prefix: a
// host that merely starts with the origin (`<run.app>.attacker.example`) is
// another origin and is left alone.

/** A 500 that is never cached and says what is missing, not its value. */
function misconfigured(what) {
  return new Response(`The ${what} is not configured.`, {
    status: 500,
    headers: { "content-type": "text/plain", "cache-control": "no-store" },
  });
}

/** The origin of the binding `value`, or `null` when it is missing or not an https URL (the secret never travels in clear). */
function originOf(value) {
  try {
    const { origin, protocol } = new URL(value);
    return protocol === "https:" ? origin : null;
  } catch {
    return null;
  }
}

/** `location` on `publicOrigin` if it points at `origin`, else `null`. */
function publicLocation(location, origin, publicOrigin) {
  let next;
  try {
    next = new URL(location, origin);
  } catch {
    return null;
  }
  if (next.origin !== origin) return null;
  return publicOrigin + next.pathname + next.search + next.hash;
}

export default {
  async fetch(request, env) {
    const secret = typeof env.ORIGIN_AUTH === "string" ? env.ORIGIN_AUTH.trim() : "";
    if (secret === "") return misconfigured("origin secret");
    const origin = originOf(env.ORIGIN);
    if (origin === null) return misconfigured("origin");
    // The public host of a rewritten `location` is the binding, never the
    // request's own Host: a forged one must not steer a redirect.
    const publicOrigin = originOf(env.PUBLIC_ORIGIN);
    if (publicOrigin === null) return misconfigured("public origin");
    const incoming = new URL(request.url);
    const target = new URL(incoming.pathname + incoming.search, origin);
    const headers = new Headers(request.headers);
    headers.delete("host");
    headers.set("x-forwarded-host", incoming.host);
    // `set` replaces any value the client sent.
    headers.set("x-origin-auth", secret);
    const response = await fetch(target, {
      method: request.method,
      headers,
      body: ["GET", "HEAD"].includes(request.method) ? undefined : request.body,
      redirect: "manual",
    });
    const location = response.headers.get("location");
    const rewritten = location ? publicLocation(location, origin, publicOrigin) : null;
    if (rewritten !== null) {
      const relayed = new Response(response.body, response);
      relayed.headers.set("location", rewritten);
      return relayed;
    }
    return response;
  },
};
