import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Runs as a container behind the nginx container, like every other site here.
  output: "standalone",
  poweredByHeader: false,
  devIndicators: false,
  // nginx and Cloudflare both compress; doing it a third time buys nothing.
  compress: false,
  reactStrictMode: true,
  typescript: { ignoreBuildErrors: false },
  async headers() {
    return [
      {
        // The converter versions these URLs with the jc-rs release, so they can
        // be cached hard without pairing a new client with an old wasm API.
        // Without this the browser refetches 1.5 MB on every navigation.
        source: "/wasm/:path*",
        headers: [
          { key: "Cache-Control", value: "public, max-age=31536000, immutable" },
        ],
      },
      {
        // X-Frame-Options, X-Content-Type-Options and Referrer-Policy are set
        // by the nginx vhost in front of this container for every site in the
        // estate. Repeating them here only produced duplicate headers on the
        // wire, so this adds the one nginx does not send.
        source: "/:path*",
        headers: [
          {
            key: "Permissions-Policy",
            value: "camera=(), geolocation=(), microphone=(), payment=(), usb=()",
          },
        ],
      },
    ];
  },
};

export default nextConfig;
