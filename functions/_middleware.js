// Password gate for the dev site (the Pages preview environment: dev.viz.palicanon.app and the
// *.palicanon-viz.pages.dev deployments). Active only where the Pages secret DEV_PASSWORD is set,
// which is the preview environment alone: production stays public. HTTP Basic auth: any user
// name, the password checked in constant time. The secret is set through the Cloudflare API from
// the macOS keychain (account palicanon, service viz-dev-password), never in a file.

const REALM = 'PaliCanon viz dev';

function sameText(a, b) {
  const x = new TextEncoder().encode(a);
  const y = new TextEncoder().encode(b);
  let diff = x.length ^ y.length;
  for (let i = 0; i < Math.max(x.length, y.length); i++) diff |= (x[i] ?? 0) ^ (y[i] ?? 0);
  return diff === 0;
}

function password(request) {
  const header = request.headers.get('Authorization') ?? '';
  if (!header.startsWith('Basic ')) return null;
  try {
    const decoded = atob(header.slice(6));
    return decoded.slice(decoded.indexOf(':') + 1);
  } catch {
    return null;
  }
}

export async function onRequest({ request, env, next }) {
  if (!env.DEV_PASSWORD) return next();
  const given = password(request);
  if (given !== null && sameText(given, env.DEV_PASSWORD)) return next();
  return new Response('Password required', {
    status: 401,
    headers: { 'WWW-Authenticate': `Basic realm="${REALM}", charset="UTF-8"`, 'Cache-Control': 'no-store' },
  });
}
