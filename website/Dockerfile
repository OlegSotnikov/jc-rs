# Next.js standalone output behind the nginx container. Nothing is published to
# the host: nginx reaches jc-rs-website:3101 over the jc-rs-system network.
#
# The base is a Docker Hardened Image. `dhi.io` is the Community registry and
# needs a plain Docker ID to pull -- the subscription only gates the `-fips`
# and `-sfw-ent` tags, which this does not use. The `-dev` variant carries the
# shell and npm the build needs; the runtime tag carries neither, which is the
# point of it.
#
# Pulling requires `docker login dhi.io`; the deploy notes say where the
# credentials live.

FROM dhi.io/node:26-alpine3.24-dev AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund

FROM dhi.io/node:26-alpine3.24-dev AS build
WORKDIR /app
ENV NEXT_TELEMETRY_DISABLED=1
COPY --from=deps /app/node_modules ./node_modules
COPY . .
# The canonical origin is baked in: it reaches metadata, sitemap and robots, all
# of which are rendered at build time.
ARG NEXT_PUBLIC_SITE_ORIGIN=https://jc-rs.com
ENV NEXT_PUBLIC_SITE_ORIGIN=$NEXT_PUBLIC_SITE_ORIGIN
RUN npm run build

FROM dhi.io/node:26-alpine3.24 AS runtime
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3101
ENV HOSTNAME=0.0.0.0
# No adduser step, and no USER line: the hardened runtime already runs as
# uid 1000 and has no shell for a RUN to use. The copies carry the ownership
# instead.
COPY --from=build --chown=1000:1000 /app/.next/standalone ./
COPY --from=build --chown=1000:1000 /app/.next/static ./.next/static
COPY --from=build --chown=1000:1000 /app/public ./public
EXPOSE 3101
CMD ["node", "server.js"]
