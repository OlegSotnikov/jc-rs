import type { MetadataRoute } from "next";
import { parsers, slugOf } from "@/lib/parsers";
import { site } from "@/lib/site";

/**
 * One entry per page, every parser included.
 *
 * Deliberately does no I/O and catches nothing: a sitemap that silently comes
 * back short is worse than one that fails, because a crawler treats a truncated
 * sitemap as the truth and drops what is missing.
 */
export default function sitemap(): MetadataRoute.Sitemap {
  const now = new Date();

  const fixed: MetadataRoute.Sitemap = [
    { url: `${site.origin}/`, changeFrequency: "weekly", priority: 1 },
    { url: `${site.origin}/parsers`, changeFrequency: "weekly", priority: 0.9 },
    { url: `${site.origin}/compatibility`, changeFrequency: "weekly", priority: 0.9 },
    { url: `${site.origin}/compare`, changeFrequency: "monthly", priority: 0.8 },
    { url: `${site.origin}/install`, changeFrequency: "monthly", priority: 0.8 },
  ];

  const pages: MetadataRoute.Sitemap = parsers.map((p) => ({
    url: `${site.origin}/parsers/${slugOf(p)}`,
    changeFrequency: "monthly" as const,
    priority: p.coverage ? 0.7 : 0.5,
  }));

  return [...fixed, ...pages].map((e) => ({ ...e, lastModified: now }));
}
