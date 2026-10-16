// From search-front/infra/worker/proxy.test.mjs. What differs: the hosts and
// the service's names, and two tests of our own at the end: the dev
// password (Authorization) reaches the origin untouched, and the gate's 401
// comes back with its WWW-Authenticate.
//
// The Worker's contract (search-front 007, design v4): it relays to ORIGIN and adds
// `x-origin-auth` from its secret `ORIGIN_AUTH`; without that secret it
// answers 500 and calls nothing (a dropped secret is loud, not a storm of
// 403s from the service); the public origin of `location` rewrites is the
// binding `PUBLIC_ORIGIN`, never the request's own Host. Run: node --test 'infra/worker/*.test.mjs'
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { afterEach, test } from "node:test";

import worker from "./proxy.js";

const ORIGIN = "https://vizualisation-dev-abc123-ew.a.run.app";
const PUBLIC_ORIGIN = "https://map.dev.palicanon.app";
const SECRET = "origin-secret-for-tests-0123456789abcdef";
const realFetch = globalThis.fetch;

/** Replace `fetch` with a recorder answering `respond(url, init)`. */
function stubFetch(respond = () => new Response("ok")) {
  const calls = [];
  globalThis.fetch = async (url, init) => {
    calls.push({ url: String(url), init });
    return respond(String(url), init);
  };
  return calls;
}

afterEach(() => {
  globalThis.fetch = realFetch;
});

const env = (extra = {}) => ({ ORIGIN, PUBLIC_ORIGIN, ORIGIN_AUTH: SECRET, ...extra });

test("adds x-origin-auth from the secret on every relayed request", async () => {
  const calls = stubFetch();
  await worker.fetch(new Request("https://map.dev.palicanon.app/?q=sati"), env());
  assert.equal(calls.length, 1);
  assert.equal(calls[0].init.headers.get("x-origin-auth"), SECRET);
  assert.equal(calls[0].url, `${ORIGIN}/?q=sati`);
});

test("overwrites an x-origin-auth sent by the client", async () => {
  const calls = stubFetch();
  const request = new Request("https://map.dev.palicanon.app/", {
    headers: { "x-origin-auth": "guess" },
  });
  await worker.fetch(request, env());
  assert.equal(calls[0].init.headers.get("x-origin-auth"), SECRET);
});

test("answers 500 and calls nothing when the secret is absent", async () => {
  const calls = stubFetch();
  const response = await worker.fetch(new Request("https://map.dev.palicanon.app/"), {
    ORIGIN,
  });
  assert.equal(response.status, 500);
  assert.equal(calls.length, 0);
});

test("answers 500 and calls nothing when the secret is blank", async () => {
  for (const blank of ["", "   ", "\n"]) {
    const calls = stubFetch();
    const response = await worker.fetch(
      new Request("https://map.dev.palicanon.app/"),
      env({ ORIGIN_AUTH: blank }),
    );
    assert.equal(response.status, 500, JSON.stringify(blank));
    assert.equal(calls.length, 0);
  }
});

test("the 500 holds no secret, no origin and is not cached", async () => {
  stubFetch();
  const response = await worker.fetch(
    new Request("https://map.dev.palicanon.app/"),
    env({ ORIGIN_AUTH: "" }),
  );
  const body = await response.text();
  assert.ok(!body.includes(ORIGIN));
  assert.ok(!body.includes(SECRET));
  assert.equal(response.headers.get("cache-control"), "no-store");
});

test("a secret with a trailing newline is sent trimmed", async () => {
  const calls = stubFetch();
  await worker.fetch(new Request("https://map.dev.palicanon.app/"), env({ ORIGIN_AUTH: `${SECRET}\n` }));
  assert.equal(calls[0].init.headers.get("x-origin-auth"), SECRET);
});

test("deletes host and sets x-forwarded-host", async () => {
  const calls = stubFetch();
  const request = new Request("https://map.dev.palicanon.app/", { headers: { host: "evil.example" } });
  await worker.fetch(request, env());
  const headers = calls[0].init.headers;
  assert.equal(headers.get("host"), null);
  assert.equal(headers.get("x-forwarded-host"), "map.dev.palicanon.app");
});

test("rewrites a Location to the run.app origin onto the public host", async () => {
  stubFetch(() => new Response(null, { status: 302, headers: { location: `${ORIGIN}/?q=x` } }));
  const response = await worker.fetch(new Request("https://map.dev.palicanon.app/old"), env());
  assert.equal(response.status, 302);
  assert.equal(response.headers.get("location"), "https://map.dev.palicanon.app/?q=x");
});

test("leaves a Location elsewhere untouched", async () => {
  stubFetch(() => new Response(null, { status: 302, headers: { location: "https://example.org/x" } }));
  const response = await worker.fetch(new Request("https://map.dev.palicanon.app/old"), env());
  assert.equal(response.headers.get("location"), "https://example.org/x");
});

test("relays the body of a POST, and none for a GET", async () => {
  const calls = stubFetch();
  const post = new Request("https://map.dev.palicanon.app/api/anything", {
    method: "POST",
    body: "x=1",
    headers: { "content-type": "application/x-www-form-urlencoded" },
  });
  await worker.fetch(post, env());
  assert.equal(calls[0].init.method, "POST");
  assert.equal(await new Response(calls[0].init.body).text(), "x=1");
  await worker.fetch(new Request("https://map.dev.palicanon.app/"), env());
  assert.equal(calls[1].init.body, undefined);
});

test("does not follow redirects (the browser does, on the public host)", async () => {
  const calls = stubFetch();
  await worker.fetch(new Request("https://map.dev.palicanon.app/"), env());
  assert.equal(calls[0].init.redirect, "manual");
});

test("does not rewrite a Location whose host only starts with the origin", async () => {
  for (const location of [
    `${ORIGIN}.attacker.example/x`,
    `${ORIGIN}@attacker.example/x`,
    `${ORIGIN}:8443/x`,
  ]) {
    stubFetch(() => new Response(null, { status: 302, headers: { location } }));
    const response = await worker.fetch(new Request("https://map.dev.palicanon.app/old"), env());
    assert.equal(response.headers.get("location"), location, location);
  }
});

test("rewrites a same-origin Location and keeps its path, query and fragment", async () => {
  stubFetch(
    () => new Response(null, { status: 301, headers: { location: `${ORIGIN}/sutta/mn1?lang=en#top` } }),
  );
  const response = await worker.fetch(new Request("https://map.dev.palicanon.app/old"), env());
  assert.equal(response.status, 301);
  assert.equal(response.headers.get("location"), "https://map.dev.palicanon.app/sutta/mn1?lang=en#top");
});

test("a relative Location stays on the public host", async () => {
  stubFetch(() => new Response(null, { status: 302, headers: { location: "/search?q=sati" } }));
  const response = await worker.fetch(new Request("https://map.dev.palicanon.app/old"), env());
  const next = new URL(response.headers.get("location"), "https://map.dev.palicanon.app/old");
  assert.equal(next.origin, "https://map.dev.palicanon.app");
  assert.equal(next.pathname + next.search, "/search?q=sati");
});

test("answers 500 and calls nothing when ORIGIN is missing or not a URL", async () => {
  for (const bad of [undefined, "", "not a url"]) {
    const calls = stubFetch();
    const response = await worker.fetch(
      new Request("https://map.dev.palicanon.app/"),
      { ORIGIN: bad, ORIGIN_AUTH: SECRET },
    );
    assert.equal(response.status, 500, String(bad));
    assert.equal(calls.length, 0);
    const body = await response.text();
    assert.ok(!body.includes(SECRET));
    assert.equal(response.headers.get("cache-control"), "no-store");
  }
});

test("refuses an http: ORIGIN: the secret never travels in clear", async () => {
  for (const bad of [
    "http://vizualisation-dev-abc123-ew.a.run.app",
    "HTTP://vizualisation-dev-abc123-ew.a.run.app/",
    "http://127.0.0.1:8080",
    "ftp://vizualisation-dev-abc123-ew.a.run.app",
    "file:///etc/passwd",
  ]) {
    const calls = stubFetch();
    const response = await worker.fetch(
      new Request("https://map.dev.palicanon.app/"),
      { ORIGIN: bad, ORIGIN_AUTH: SECRET },
    );
    assert.equal(response.status, 500, bad);
    assert.equal(calls.length, 0, bad);
    const body = await response.text();
    assert.ok(!body.includes(SECRET), bad);
    assert.equal(response.headers.get("cache-control"), "no-store", bad);
  }
});

test("an x-origin-auth sent by the client never reaches the origin, in any case or number", async () => {
  const calls = stubFetch();
  const sent = new Headers({ "x-origin-auth": "guess" });
  sent.append("X-Origin-Auth", "another guess");
  await worker.fetch(
    new Request("https://map.dev.palicanon.app/", { headers: sent }),
    env(),
  );
  const upstream = [...calls[0].init.headers].filter(([name]) => name.toLowerCase() === "x-origin-auth");
  assert.deepEqual(upstream, [["x-origin-auth", SECRET]]);
});

test("rewrites a Location onto PUBLIC_ORIGIN, whatever Host the request forges", async () => {
  for (const [url, host] of [
    ["https://evil.example/old", undefined],
    ["https://map.dev.palicanon.app/old", "evil.example"],
    ["https://evil.example:8443/old?x=1", "evil.example:8443"],
  ]) {
    stubFetch(() => new Response(null, { status: 302, headers: { location: `${ORIGIN}/sutta/mn1?lang=en#top` } }));
    const request = new Request(url, host ? { headers: { host } } : {});
    const response = await worker.fetch(request, env());
    assert.equal(
      response.headers.get("location"),
      `${PUBLIC_ORIGIN}/sutta/mn1?lang=en#top`,
      `${url} host=${host}`,
    );
  }
});

test("a Location to the root of the origin is rewritten onto PUBLIC_ORIGIN too", async () => {
  stubFetch(() => new Response(null, { status: 302, headers: { location: `${ORIGIN}/` } }));
  const response = await worker.fetch(new Request("https://evil.example/old"), env());
  assert.equal(response.headers.get("location"), `${PUBLIC_ORIGIN}/`);
});

test("answers 500 and calls nothing when PUBLIC_ORIGIN is missing, not https or not a URL", async () => {
  for (const bad of [
    undefined,
    "",
    "not a url",
    "http://map.dev.palicanon.app",
    "HTTP://map.dev.palicanon.app/",
    "ftp://map.dev.palicanon.app",
    "file:///etc/passwd",
  ]) {
    const calls = stubFetch();
    const response = await worker.fetch(
      new Request("https://map.dev.palicanon.app/"),
      { ORIGIN, PUBLIC_ORIGIN: bad, ORIGIN_AUTH: SECRET },
    );
    assert.equal(response.status, 500, String(bad));
    assert.equal(calls.length, 0, String(bad));
    const body = await response.text();
    assert.equal(body, "The public origin is not configured.", String(bad));
    assert.ok(!body.includes(SECRET), String(bad));
    assert.equal(response.headers.get("cache-control"), "no-store", String(bad));
  }
});

test("answers 500 when PUBLIC_ORIGIN is absent altogether", async () => {
  const calls = stubFetch();
  const response = await worker.fetch(new Request("https://map.dev.palicanon.app/"), {
    ORIGIN,
    ORIGIN_AUTH: SECRET,
  });
  assert.equal(response.status, 500);
  assert.equal(calls.length, 0);
});

test("relays the visitor's Authorization (the dev password) to the origin untouched", async () => {
  const calls = stubFetch();
  const basic = "Basic ZGVwbG95OmRldi1wYXNzd29yZA==";
  await worker.fetch(new Request("https://map.dev.palicanon.app/", { headers: { authorization: basic } }), env());
  assert.equal(calls[0].init.headers.get("authorization"), basic);
  assert.equal(calls[0].init.headers.get("x-origin-auth"), SECRET);
});

test("relays the gate's 401 with its WWW-Authenticate, so the browser asks for the password", async () => {
  stubFetch(
    () =>
      new Response(null, {
        status: 401,
        headers: { "www-authenticate": 'Basic realm="PaliCanon viz dev"', "cache-control": "private, no-store" },
      }),
  );
  const response = await worker.fetch(new Request("https://map.dev.palicanon.app/"), env());
  assert.equal(response.status, 401);
  assert.equal(response.headers.get("www-authenticate"), 'Basic realm="PaliCanon viz dev"');
  assert.equal(response.headers.get("cache-control"), "private, no-store");
});

// Task 013 (urgent, user 2026-10-16; the finding is anatta-sso 066's review
// delta 1, confirmed by a run in anatta, message 10754): `new URL(pathname +
// search, origin)` RESOLVES the path, and a path starting `//` (or `/\`,
// which the URL parser reads as `//`) is a scheme-relative reference — the
// Worker fetched the attacker's host carrying x-origin-auth and every client
// header. The Worker now sets the path on ORIGIN and refuses such a path
// outright. These four tests are anatta-console 068's and anatta-sso 066's,
// on this service's host.

/** The request paths that try to leave ORIGIN's host. */
const ESCAPES = [
  "//attacker.example/x",
  "/\\attacker.example/x",
  "/%5Cattacker.example/x",
  "//",
];

/** A request-like object whose `url` is the raw string, as no `Request` would normalise it. */
const raw = (url) => ({ url, method: "GET", headers: new Headers() });

test("no request path sends the fetch, or the secret, to another host", async () => {
  const host = new URL(ORIGIN).host;
  for (const path of ESCAPES) {
    for (const request of [new Request(`https://map.dev.palicanon.app${path}`), raw(`https://map.dev.palicanon.app${path}`)]) {
      const calls = stubFetch();
      const response = await worker.fetch(request, env());
      for (const call of calls) {
        const elsewhere = new URL(call.url).host !== host;
        assert.ok(
          !(elsewhere && call.init.headers.get("x-origin-auth") !== null),
          `${JSON.stringify(path)} sent x-origin-auth to ${call.url}`,
        );
        assert.equal(new URL(call.url).host, host, `${JSON.stringify(path)} fetched ${call.url}`);
      }
      assert.ok(calls.length === 1 || response.status === 400, `${JSON.stringify(path)}: ${response.status}`);
    }
  }
});

test("a path that would leave ORIGIN's host is a 400, never fetched", async () => {
  for (const path of ["//attacker.example/x", "/\\attacker.example/x", "//"]) {
    for (const request of [new Request(`https://map.dev.palicanon.app${path}`), raw(`https://map.dev.palicanon.app${path}`)]) {
      const calls = stubFetch();
      const response = await worker.fetch(request, env());
      assert.equal(response.status, 400, JSON.stringify(path));
      assert.equal(calls.length, 0, JSON.stringify(path));
      assert.equal(response.headers.get("cache-control"), "no-store");
      assert.ok(!(await response.text()).includes(SECRET));
    }
  }
});

test("an encoded backslash stays a path on ORIGIN", async () => {
  const calls = stubFetch();
  await worker.fetch(new Request("https://map.dev.palicanon.app/%5Cattacker.example/x?q=1"), env());
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, `${ORIGIN}/%5Cattacker.example/x?q=1`);
  assert.equal(calls[0].init.headers.get("x-origin-auth"), SECRET);
});

test("the target is ORIGIN with the path set, never the path resolved against it", async () => {
  // The 400 above refuses every path a resolution would take elsewhere, so a
  // target resolved again behind it behaves the same: this reads the source.
  const source = await readFile(new URL("./proxy.js", import.meta.url), "utf8");
  assert.ok(source.includes("target.pathname = incoming.pathname;"), "proxy.js does not set the target's path");
  assert.ok(source.includes("target.search = incoming.search;"), "proxy.js does not set the target's query");
  assert.ok(
    !/new URL\([^)]*pathname[^)]*,\s*(origin|env\.ORIGIN)\)/.test(source),
    "proxy.js resolves a path against the origin",
  );
});

test("a body dropped on GET and HEAD drops its length headers too", async () => {
  // A content-length or transfer-encoding left on a request with no body
  // makes the origin wait for bytes that never come (anatta review 066, #10900).
  for (const method of ["GET", "HEAD"]) {
    const calls = stubFetch();
    const request = new Request("https://map.dev.palicanon.app/", {
      method,
      headers: { "content-length": "5", "transfer-encoding": "chunked", "x-keep": "yes" },
    });
    assert.equal(request.headers.get("content-length"), "5", "the request must carry the header for the test to mean anything");
    await worker.fetch(request, env());
    const sent = calls[0].init.headers;
    assert.equal(sent.get("content-length"), null, method);
    assert.equal(sent.get("transfer-encoding"), null, method);
    assert.equal(sent.get("x-keep"), "yes", method);
    assert.equal(calls[0].init.body, undefined, method);
  }
});

test("a POST keeps its body and its content-length", async () => {
  const calls = stubFetch();
  const request = new Request("https://map.dev.palicanon.app/", {
    method: "POST",
    headers: { "content-length": "3", "content-type": "application/x-www-form-urlencoded" },
    body: "x=1",
  });
  await worker.fetch(request, env());
  assert.equal(calls[0].init.headers.get("content-length"), "3");
  assert.equal(await new Response(calls[0].init.body).text(), "x=1");
});

// Review round 1 (T1, T2, T3): a path that only LOOKS like an escape, or a
// `//` anywhere but the start of the path, is a legitimate request — relayed
// to ORIGIN as is, never refused.

test("an encoded backslash is fetched on ORIGIN, as a Request and as a raw url", async () => {
  const host = new URL(ORIGIN).host;
  const url = "https://map.dev.palicanon.app/%5Cattacker.example/x?q=1";
  for (const request of [new Request(url), raw(url)]) {
    const calls = stubFetch();
    const response = await worker.fetch(request, env());
    assert.notEqual(response.status, 400);
    assert.equal(calls.length, 1);
    assert.equal(new URL(calls[0].url).host, host);
    assert.equal(calls[0].url, `${ORIGIN}/%5Cattacker.example/x?q=1`);
    assert.equal(calls[0].init.headers.get("x-origin-auth"), SECRET);
  }
});

test("a // later in the path or in the query is relayed unchanged to ORIGIN", async () => {
  for (const path of ["/a//b", "/?next=//x", "/a//b?next=//x&to=https://example.org//y"]) {
    for (const request of [new Request(`https://map.dev.palicanon.app${path}`), raw(`https://map.dev.palicanon.app${path}`)]) {
      const calls = stubFetch();
      const response = await worker.fetch(request, env());
      assert.notEqual(response.status, 400, path);
      assert.equal(calls.length, 1, path);
      assert.equal(calls[0].url, `${ORIGIN}${path}`, path);
    }
  }
});
