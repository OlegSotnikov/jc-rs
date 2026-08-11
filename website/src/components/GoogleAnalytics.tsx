"use client";

import { Suspense, useEffect, useRef } from "react";
import Script from "next/script";
import { usePathname, useSearchParams } from "next/navigation";

const measurementId = process.env.NEXT_PUBLIC_GA_MEASUREMENT_ID;
const validMeasurementId =
  measurementId && /^G-[A-Z0-9]+$/.test(measurementId) ? measurementId : undefined;

declare global {
  interface Window {
    dataLayer: unknown[];
    gtag: (...args: unknown[]) => void;
  }
}

function PageViewTracker() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const query = searchParams.toString();
  const previousLocation = useRef<string | undefined>(undefined);

  useEffect(() => {
    if (!validMeasurementId || typeof window.gtag !== "function") {
      return;
    }

    const pageLocation = window.location.href;
    window.gtag("event", "page_view", {
      page_location: pageLocation,
      page_path: `${pathname}${query ? `?${query}` : ""}`,
      page_referrer: previousLocation.current ?? document.referrer,
      page_title: document.title,
      send_to: validMeasurementId,
    });
    previousLocation.current = pageLocation;
  }, [pathname, query]);

  return null;
}

export function GoogleAnalytics() {
  if (!validMeasurementId) {
    return null;
  }

  return (
    <>
      <script
        dangerouslySetInnerHTML={{
          __html: `
            window.dataLayer = window.dataLayer || [];
            function gtag(){dataLayer.push(arguments);}
            gtag('js', new Date());
            gtag('config', ${JSON.stringify(validMeasurementId)}, {send_page_view: false});
          `,
        }}
      />
      <Script
        src={`https://www.googletagmanager.com/gtag/js?id=${validMeasurementId}`}
        strategy="afterInteractive"
      />
      <Suspense fallback={null}>
        <PageViewTracker />
      </Suspense>
    </>
  );
}
