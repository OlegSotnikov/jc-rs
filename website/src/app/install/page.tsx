import type { Metadata } from "next";
import { CopyLine } from "@/components/CopyLine";
import { install, site } from "@/lib/site";
import { summary } from "@/lib/parsers";

const description =
  "Static binaries for five targets, five crates on crates.io, a scratch Docker image for amd64 and arm64, a Homebrew tap, and a WebAssembly build on npm.";

export const metadata: Metadata = {
  title: "Install",
  description,
  alternates: { canonical: "/install" },
  openGraph: {
    title: "Install jc-rs",
    description,
    url: `${site.origin}/install`,
    images: [site.socialImage],
  },
  twitter: {
    card: "summary_large_image",
    title: "Install jc-rs",
    description,
    images: [site.socialImage.url],
  },
};

export default function Install() {
  return (
    <div className="mx-auto max-w-4xl px-5 py-14">
      <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
        v{summary.version}
      </p>
      <h1 className="mt-3 text-4xl">Install</h1>
      <p className="mt-4 max-w-2xl text-lg text-[var(--color-muted)]">
        Every channel is cut from the same git tag by one workflow. Nothing is built by hand.
      </p>

      <div className="mt-8 grid gap-3 sm:grid-cols-2">
        {install.map((i) => (
          <CopyLine key={i.cmd} label={i.label} command={i.cmd} />
        ))}
      </div>

      <section className="mt-14">
        <h2 className="text-2xl">The binary is jc-rs, not jc</h2>
        <p className="mt-3 text-[var(--color-muted)]">
          Release archives carry a <code className="font-mono text-sm">jc</code> alias, and
          nothing installs it for you. Dropping a second <code className="font-mono text-sm">jc</code>{" "}
          into <code className="font-mono text-sm">PATH</code> would shadow the original, which
          is somebody else&apos;s tool and the authority this one is measured against. Enable
          it deliberately or not at all.
        </p>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Container</h2>
        <p className="mt-3 text-[var(--color-muted)]">
          A compact <code className="font-mono text-sm">scratch</code> image contains the binary
          and its licence, with no shell, no libc and no package manager.{" "}
          <code className="font-mono text-sm">linux/amd64</code> and{" "}
          <code className="font-mono text-sm">linux/arm64</code> ship in one manifest, so{" "}
          <code className="font-mono text-sm">docker pull</code> picks the right one.
        </p>
        <pre className="mt-4 overflow-x-auto rounded-lg border bg-[var(--color-sunk)] p-4 font-mono text-xs leading-relaxed">
          {`ps aux | docker run --rm -i appmasterio/jc-rs --ps | jq '.[0]'
tail -f access.log | docker run --rm -i appmasterio/jc-rs -u --clf-s`}
        </pre>
        <p className="mt-3 text-sm text-[var(--color-faint)]">
          Magic syntax (<code className="font-mono">jc-rs df -h</code>) needs the command
          inside the container, and this image deliberately has nothing else in it. Pipe
          instead, or use the standalone binary.
        </p>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Shell completions</h2>
        <pre className="mt-4 overflow-x-auto rounded-lg border bg-[var(--color-sunk)] p-4 font-mono text-xs leading-relaxed">
          {`jc-rs -B > /etc/bash_completion.d/jc-rs
jc-rs -Z > "\${fpath[1]}/_jc-rs"
jc-rs -F > ~/.config/fish/completions/jc-rs.fish`}
        </pre>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">As a library</h2>
        <p className="mt-3 text-[var(--color-muted)]">
          The parsers are their own crate. Parsers register themselves at link time, so
          depending on <code className="font-mono text-sm">jc-rs-parsers</code> is what fills
          the registry.
        </p>
        <pre className="mt-4 overflow-x-auto rounded-lg border bg-[var(--color-sunk)] p-4 font-mono text-xs leading-relaxed">
          {`let output = jc_rs_parsers::parse("df", df_output)?;

// Streaming parsers hand back a session you feed a line at a time.
let mut session = jc_rs_parsers::session("clf_s").unwrap();
for line in reader.lines() {
    if let Some(record) = session.parse_line(&line?, true)? {
        handle(record);
    }
}`}
        </pre>
        <p className="mt-4 text-sm text-[var(--color-faint)]">
          The same parsers compile to WebAssembly (
          <a href={site.npm} className="text-[var(--color-key)] underline-offset-4 hover:underline">
            jc-rs-wasm
          </a>
          ), which is what runs the converter on this site&apos;s front page.
        </p>
      </section>
    </div>
  );
}
