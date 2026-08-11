"use client";

import Script from "next/script";

const measurementId = process.env.NEXT_PUBLIC_GA_MEASUREMENT_ID;
const validMeasurementId =
  measurementId && /^G-[A-Z0-9]+$/.test(measurementId) ? measurementId : undefined;

export function GoogleAnalytics() {
  if (!validMeasurementId) {
    return null;
  }

  return (
    <>
      <Script
        src={`https://www.googletagmanager.com/gtag/js?id=${validMeasurementId}`}
        strategy="afterInteractive"
      />
      <Script id="google-analytics" strategy="afterInteractive">
        {`
          window.dataLayer = window.dataLayer || [];
          function gtag(){dataLayer.push(arguments);}
          gtag('js', new Date());
          gtag('config', ${JSON.stringify(validMeasurementId)});
        `}
      </Script>
    </>
  );
}
