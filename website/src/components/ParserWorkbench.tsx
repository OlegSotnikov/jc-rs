"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Panes } from "@/components/Panes";
import {
  loadJcRs,
  MAX_INTERACTIVE_INPUT_CHARACTERS,
  runParse,
  type JcRs,
} from "@/lib/jcrs";

const MAX_FILE_BYTES = 2 * 1024 * 1024;
const PARSE_DELAY_MS = 120;

type View = {
  input: string;
  output: string;
  error: string | null;
  micros: number | null;
};

export function ParserWorkbench({
  parser,
  argument,
  sampleInput,
  sampleOutput,
  accept,
}: {
  parser: string;
  argument: string;
  sampleInput: string;
  sampleOutput: string;
  accept?: string;
}) {
  const [view, setView] = useState<View>({
    input: sampleInput,
    output: sampleOutput,
    error: null,
    micros: null,
  });
  const [live, setLive] = useState(false);
  const [failed, setFailed] = useState(false);
  const [parsing, setParsing] = useState(false);
  const [copied, setCopied] = useState(false);
  const moduleRef = useRef<JcRs | null>(null);
  const loadingRef = useRef(false);
  const copyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const parseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const parse = useCallback(
    (input: string, module: JcRs | null): View => {
      if (!input.trim()) return { input, output: "", error: null, micros: null };
      if (!module) return { input, output: "", error: null, micros: null };
      if (input.length > MAX_INTERACTIVE_INPUT_CHARACTERS) {
        return {
          input,
          output: "",
          error:
            "Interactive input is limited to 3 million characters. Use the jc-rs CLI for larger data.",
          micros: null,
        };
      }
      const result = runParse(module, parser, input);
      if (
        result.ok &&
        result.json === "[]" &&
        (parser === "x509_cert" || parser === "x509_csr")
      ) {
        return {
          input,
          output: "",
          error:
            parser === "x509_cert"
              ? "No valid X.509 certificate was found in this PEM or DER input."
              : "No complete PEM certificate request was found in this text.",
          micros: null,
        };
      }
      return result.ok
        ? { input, output: result.json, error: null, micros: result.micros }
        : { input, output: "", error: result.error, micros: null };
    },
    [parser],
  );

  const warm = useCallback(async () => {
    if (moduleRef.current || loadingRef.current) return;
    loadingRef.current = true;
    setFailed(false);
    try {
      const wasm = await loadJcRs();
      moduleRef.current = wasm;
      setLive(true);
      setView((current) => parse(current.input, wasm));
    } catch {
      setFailed(true);
      setLive(false);
    } finally {
      loadingRef.current = false;
      setParsing(false);
    }
  }, [parse]);

  useEffect(() => {
    const idle = "requestIdleCallback" in window;
    const id = idle
      ? requestIdleCallback(() => void warm())
      : window.setTimeout(() => void warm(), 1200);
    return () => {
      if (idle) cancelIdleCallback(id);
      else window.clearTimeout(id);
      if (copyTimer.current) clearTimeout(copyTimer.current);
      if (parseTimer.current) clearTimeout(parseTimer.current);
    };
  }, [warm]);

  function edit(input: string) {
    void warm();
    setCopied(false);
    if (parseTimer.current) clearTimeout(parseTimer.current);

    if (!moduleRef.current) {
      setParsing(false);
      setView({ input, output: "", error: null, micros: null });
      return;
    }

    if (!input.trim()) {
      setParsing(false);
      setView(parse(input, moduleRef.current));
      return;
    }

    setParsing(true);
    setView({ input, output: "", error: null, micros: null });
    parseTimer.current = setTimeout(() => {
      const wasm = moduleRef.current;
      if (wasm) {
        setView((current) => (current.input === input ? parse(input, wasm) : current));
      }
      setParsing(false);
      parseTimer.current = null;
    }, PARSE_DELAY_MS);
  }

  async function upload(file: File | undefined) {
    if (!file) return;
    if (parseTimer.current) clearTimeout(parseTimer.current);
    parseTimer.current = null;
    setParsing(false);
    if (file.size > MAX_FILE_BYTES) {
      setView({
        input: "",
        output: "",
        error:
          parser === "x509_cert"
            ? "This browser tool accepts certificate files up to 2 MiB. For larger PEM files, use the jc-rs CLI; convert binary DER to PEM first."
            : "This browser tool accepts files up to 2 MiB. Use the jc-rs CLI for larger data.",
        micros: null,
      });
      return;
    }
    try {
      const input =
        parser === "x509_cert" ? await certificateFileInput(file) : await file.text();
      if (!input) {
        setView({ input: "", output: "", error: "That file is empty.", micros: null });
        return;
      }
      const formatError = textUploadError(parser, input);
      if (formatError) {
        setView({ input: "", output: "", error: formatError, micros: null });
        return;
      }
      edit(input);
    } catch {
      setParsing(false);
      setView((current) => ({
        ...current,
        output: "",
        error: "The browser could not read that file as supported input.",
        micros: null,
      }));
    }
  }

  function restoreSample() {
    if (parseTimer.current) clearTimeout(parseTimer.current);
    parseTimer.current = null;
    setParsing(false);
    setCopied(false);
    setView(
      moduleRef.current
        ? parse(sampleInput, moduleRef.current)
        : { input: sampleInput, output: sampleOutput, error: null, micros: null },
    );
  }

  async function copyOutput() {
    if (!view.output) return;
    try {
      await navigator.clipboard.writeText(view.output);
      setCopied(true);
      if (copyTimer.current) clearTimeout(copyTimer.current);
      copyTimer.current = setTimeout(() => setCopied(false), 1800);
    } catch {
      setCopied(false);
    }
  }

  function downloadOutput() {
    if (!view.output) return;
    const url = URL.createObjectURL(
      new Blob([`${view.output}\n`], { type: "application/json;charset=utf-8" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${parser.replaceAll("_", "-")}.json`;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
  }

  const status = view.error
    ? "input does not match"
    : parsing
      ? "parsing · local"
      : live && view.micros !== null
        ? `${(view.micros / 1000).toFixed(view.micros < 1000 ? 2 : 1)} ms · local`
        : live
          ? "ready · local"
          : failed
            ? "sample shown · WebAssembly unavailable"
            : "loading local parser";
  const announcedStatus = failed
    ? "The local WebAssembly parser is unavailable. Edit the input to retry."
    : live
      ? "Local parser ready."
      : "Loading the local parser.";
  const fileControlLabel =
    parser === "x509_cert" ? "Open certificate file" : "Open text file";

  const controlClass =
    "rounded-md border px-3 py-1.5 font-mono text-xs text-[var(--color-muted)] transition-colors hover:border-[var(--color-key)] hover:text-[var(--color-ink)] disabled:cursor-not-allowed disabled:opacity-45";

  return (
    <div className="rise">
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <label
          className={`${controlClass} relative cursor-pointer focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-[var(--color-key)]`}
        >
          {fileControlLabel}
          <input
            type="file"
            accept={accept}
            aria-label={`${fileControlLabel} to parse`}
            className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
            onChange={(event) => {
              void upload(event.currentTarget.files?.[0]);
              event.currentTarget.value = "";
            }}
          />
        </label>
        <button
          type="button"
          className={controlClass}
          disabled={!sampleInput}
          onClick={restoreSample}
        >
          Restore sample
        </button>
        <span className="mx-1 hidden h-5 border-l sm:block" aria-hidden="true" />
        <button
          type="button"
          className={controlClass}
          onClick={() => void copyOutput()}
          disabled={!view.output}
        >
          {copied ? "Copied" : "Copy JSON"}
        </button>
        <button
          type="button"
          className={controlClass}
          onClick={downloadOutput}
          disabled={!view.output}
        >
          Download .json
        </button>
        <span
          aria-hidden="true"
          className="ml-auto font-mono text-[11px] text-[var(--color-faint)]"
        >
          {status}
        </span>
        <span role="status" aria-live="polite" className="sr-only">
          {announcedStatus}
        </span>
      </div>

      <Panes
        input={view.input}
        output={view.output}
        error={view.error}
        busy={parsing || (!live && view.input !== sampleInput)}
        onInputChange={edit}
        inputAriaLabel={`Input for the ${parser.replaceAll("_", "-")} parser`}
        inputLabel={
          <>
            <span>input</span>
            <span className="normal-case">editable</span>
          </>
        }
        outputLabel={
          <>
            <span>jc-rs {argument}</span>
            <span className="normal-case">JSON</span>
          </>
        }
      />

      <p className="mt-3 text-sm text-[var(--color-muted)]">
        Hover over a JSON value, or focus the JSON pane and use the Left and Right arrow keys, to
        trace it to the input. Parsing runs inside this tab in WebAssembly. Pasted text and opened
        files are not uploaded. If site analytics is enabled, it may receive page-level visit data
        such as the URL, title, and referrer; text in this editor is never included.
      </p>
    </div>
  );
}

function textUploadError(parser: string, input: string): string | null {
  if (
    parser === "x509_csr" &&
    !input.includes("-----BEGIN CERTIFICATE REQUEST-----") &&
    !input.includes("-----BEGIN NEW CERTIFICATE REQUEST-----")
  ) {
    return "This browser tool accepts PEM certificate-request text. Binary DER files are not supported.";
  }
  if (parser === "plist" && (input.startsWith("bplist") || input.includes("\uFFFD"))) {
    return "This browser tool accepts XML property-list text. Binary plist files are not supported.";
  }
  return null;
}

async function certificateFileInput(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  if (bytes.length === 0) return "";

  try {
    const text = new TextDecoder("utf-8", { fatal: true })
      .decode(bytes)
      .replace(/^\uFEFF/, "");
    if (text.includes("-----BEGIN CERTIFICATE-----")) return text;
  } catch {
    // Binary DER is expected to fail UTF-8 decoding. It is wrapped below.
  }

  let binary = "";
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + chunkSize));
  }
  const lines = btoa(binary).match(/.{1,64}/g)?.join("\n") ?? "";
  return `-----BEGIN CERTIFICATE-----\n${lines}\n-----END CERTIFICATE-----`;
}
