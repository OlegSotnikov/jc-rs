import Link from "next/link";
import { summary } from "@/lib/parsers";

export default function NotFound() {
  return (
    <div className="mx-auto max-w-2xl px-5 py-24">
      <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">404</p>
      <h1 className="mt-3 text-4xl">No parser at this address</h1>
      <p className="mt-4 text-[var(--color-muted)]">
        jc-rs ships {summary.documented} parsers and each has its own page. If you were
        looking for one, the index lists them all with search.
      </p>
      <div className="mt-8 flex gap-3">
        <Link href="/parsers" className="rounded-md border px-4 py-2 text-sm transition-colors hover:border-[var(--color-key)]">
          Browse parsers
        </Link>
        <Link href="/" className="rounded-md border px-4 py-2 text-sm transition-colors hover:border-[var(--color-key)]">
          Home
        </Link>
      </div>
    </div>
  );
}
