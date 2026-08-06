import type { Metadata } from "next";
import { Bricolage_Grotesque, Inter, JetBrains_Mono } from "next/font/google";
import { SiteHeader } from "@/components/SiteHeader";
import { SiteFooter } from "@/components/SiteFooter";
import { site } from "@/lib/site";
import summary from "@/data/summary.json";
import "./globals.css";

const bricolage = Bricolage_Grotesque({
  subsets: ["latin"],
  variable: "--font-bricolage",
  display: "swap",
});
const inter = Inter({ subsets: ["latin"], variable: "--font-inter", display: "swap" });
const jetbrains = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains",
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL(site.origin),
  title: {
    default: `jc-rs — ${site.tagline}`,
    template: "%s — jc-rs",
  },
  description:
    `Convert the output of ${summary.documented} command-line tools, file formats and strings to JSON. ` +
    `One static Rust binary, no runtime to install, ${summary.matchRate}% verified against the full reference corpus on every commit.`,
  applicationName: "jc-rs",
  authors: [{ name: "Oleg Sotnikov" }],
  creator: "Oleg Sotnikov",
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
  formatDetection: { telephone: false, address: false, email: false },
  referrer: "origin-when-cross-origin",
  openGraph: {
    type: "website",
    siteName: "jc-rs",
    url: site.origin,
    title: `jc-rs — ${site.tagline}`,
    description: `${summary.documented} parsers in one static binary. Verified against ${summary.tested} reference fixture pairs on every commit, and the number is published whatever it says.`,
  },
  twitter: {
    card: "summary_large_image",
    title: `jc-rs — ${site.tagline}`,
    description: `${summary.documented} parsers in one static binary, verified over ${summary.tested} reference fixture pairs.`,
  },
  alternates: { canonical: "/" },
};

/** Set the theme before first paint so a dark-mode reader never sees a flash. */
const THEME_BOOTSTRAP = `(()=>{try{const t=localStorage.getItem("jcrs-theme");if(t)document.documentElement.dataset.theme=t}catch{}})()`;

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={`${bricolage.variable} ${inter.variable} ${jetbrains.variable}`}
    >
      <body className="min-h-screen antialiased">
        <script dangerouslySetInnerHTML={{ __html: THEME_BOOTSTRAP }} />
        <a
          href="#main"
          className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-[var(--color-surface)] focus:px-4 focus:py-2 focus:text-sm"
        >
          Skip to content
        </a>
        <SiteHeader />
        <main id="main">{children}</main>
        <SiteFooter />
      </body>
    </html>
  );
}
