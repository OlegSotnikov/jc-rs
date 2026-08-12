import { ImageResponse } from "next/og";
import summary from "@/data/summary.json";

export const alt = `jc-rs — ${summary.documented} parsers in one static binary`;
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

/**
 * The card is the measurement, because that is the claim. No logo, no gradient:
 * the number and what it is over.
 */
export default function OpengraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          background: "#0b0e13",
          color: "#e7ecf3",
          padding: 72,
          fontFamily: "monospace",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 26, color: "#6d7b90" }}>
          <span>jc-rs.com</span>
          <span>v{summary.version}</span>
        </div>

        <div style={{ display: "flex", flexDirection: "column" }}>
          <div style={{ display: "flex", alignItems: "flex-end", gap: 28 }}>
            <span style={{ fontSize: 190, lineHeight: 0.85, fontWeight: 700 }}>
              {summary.matchRate}%
            </span>
            <span style={{ fontSize: 34, color: "#7aa2f7", paddingBottom: 22 }}>
              verified
            </span>
          </div>
          <div style={{ display: "flex", height: 6, marginTop: 34, background: "#212936" }}>
            <div style={{ width: "100%", background: "#6ed3a3" }} />
          </div>
          <span style={{ fontSize: 32, color: "#97a3b6", marginTop: 28 }}>
            {summary.matched} of {summary.tested} structural JSON matches
          </span>
        </div>

        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 28, color: "#6d7b90" }}>
          <span>{summary.documented} parsers · command output to JSON</span>
          <span>one static binary</span>
        </div>
      </div>
    ),
    size,
  );
}
