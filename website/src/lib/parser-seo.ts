export const parserSeoNames = [
  "jwt",
  "x509_cert",
  "x509_csr",
  "xml",
  "csv",
  "yaml",
  "url",
  "toml",
  "tsv",
  "plist",
  "asciitable",
  "asciitable_m",
] as const;

export type ParserSeoName = (typeof parserSeoNames)[number];

export type ParserSeoGuide = {
  href: `/guides/${string}`;
  label: string;
};

/**
 * Editorial copy for parser pages with a distinct search intent.
 *
 * `sampleInput` and `sampleOutput` are always a pair. Every pair in this file
 * has been run through the corresponding jc-rs parser; neither side is a
 * hand-written approximation of the parser's behavior.
 */
type ParserSeoSample =
  | { sampleInput: string; sampleOutput: string }
  | { sampleInput?: never; sampleOutput?: never };

export type ParserSeoContent = {
  title: string;
  description: string;
  eyebrow: string;
  intro: string;
  privacyAndValidation: string;
  acceptedInput: string;
  outputDetails: readonly string[];
  queryLanguage: readonly string[];
  relatedGuides: readonly ParserSeoGuide[];
} & ParserSeoSample;

export const parserSeo: Readonly<Record<ParserSeoName, ParserSeoContent>> = {
  jwt: {
    title: "JWT Decoder: Read a Token Locally",
    description:
      "Decode a three-part JWT into its JSON header, payload, and signature bytes in your browser. Decoding does not verify the signature.",
    eyebrow: "Private JWT decoder",
    intro:
      "A compact JSON Web Token carries a header, payload, and signature. Paste one here to decode the first two parts as JSON and render the signature bytes as readable hex, all inside the browser parser.",
    privacyAndValidation:
      "The WebAssembly parser processes the supplied text without network or storage access. It decodes the token but does not verify its signature with a secret or public key, enforce an algorithm policy, or validate expiration, issuer, audience, or other claims. Decoded does not mean trusted.",
    acceptedInput:
      "A compact JWT with three dot-separated parts: an unpadded base64url header, an unpadded base64url payload, and a base64url signature. The first two parts must decode to valid UTF-8 JSON. Leading and trailing whitespace is ignored.",
    outputDetails: [
      "`header` contains the decoded JOSE header as a JSON value; fields such as `alg`, `typ`, and `kid` appear when the token carries them.",
      "`payload` contains the decoded claims without evaluating or normalizing them.",
      "`signature` is the decoded signature byte sequence rendered as lowercase, colon-delimited hex.",
      "Invalid base64url, non-UTF-8 text, invalid JSON, or the wrong compact-token shape produces a parse error.",
    ],
    queryLanguage: [
      "Decode a JWT token into header, payload, and signature data",
      "Inspect compact-token structure with the JWT parser",
      "Use the JWT decoder online; the token stays inside the browser parser",
      "Read JWT claims without verifying or trusting them",
      "Distinguish JWT parsing from signature validation",
    ],
    sampleInput:
      "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
    sampleOutput: `{
  "header": {
    "alg": "HS256",
    "typ": "JWT"
  },
  "payload": {
    "iat": 1516239022,
    "name": "John Doe",
    "sub": "1234567890"
  },
  "signature": "49:f9:4a:c7:04:49:48:c7:8a:28:5d:90:4f:87:f0:a4:c7:89:7f:7e:8f:3a:4e:b2:25:5f:da:75:0b:2c:c3:97"
}`,
    relatedGuides: [
      { href: "/guides/bash-jc-rs-jq", label: "Filter decoded fields with jq" },
    ],
  },

  x509_cert: {
    title: "CRT Viewer and X.509 Certificate Decoder",
    description:
      "Open PEM or binary DER X.509 certificates (.crt, .cer, .der) locally and inspect subject, issuer, validity, keys, extensions, and signatures as JSON.",
    eyebrow: "Private certificate decoder",
    intro:
      "Open a PEM or binary DER certificate and inspect its serial number, distinguished names, validity period, public key, and recognized X.509 extensions as structured JSON.",
    privacyAndValidation:
      "Certificate parsing happens locally in the browser. The result is a field view, not a trust decision: jc-rs does not verify the signature, build a certificate chain, check revocation, match a hostname, or decide whether the certificate is currently valid.",
    acceptedInput:
      "Paste one or more complete `-----BEGIN CERTIFICATE-----` PEM blocks, or open a single binary DER file with a `.crt`, `.cer`, or `.der` extension. For DER, the browser wraps the bytes as PEM inside this tab before calling the same parser. A PEM bundle may contain unrelated material; only certificate blocks are selected. CLI stdin is UTF-8 text, so pipe PEM to `jc-rs --x509-cert` or convert binary DER with OpenSSL first. This tool does not accept PKCS#7, P7B, PFX, P12, private keys, CSRs, or CRLs as certificates.",
    outputDetails: [
      "Each decoded certificate becomes one item in an array, so a PEM bundle can produce several records.",
      "`tbs_certificate` includes version, hexadecimal and decimal serial forms, issuer, subject, validity, public-key information, and extensions.",
      "Validity dates are exposed as Unix timestamps and ISO strings; jc-rs reports them but does not compare them with the current time.",
      "Signature algorithm metadata and the certificate signature bytes are exposed for inspection, not cryptographic verification. Malformed blocks may yield no record rather than a trust verdict.",
    ],
    queryLanguage: [
      "Read an X.509 certificate as structured fields",
      "View PEM certificates and DER-encoded .crt, .cer, or .der files as JSON",
      "Inspect certificate subject, issuer, validity, and extensions",
      "Use an SSL certificate decoder without running a trust check",
      "Inspect certificate fields without invoking OpenSSL",
    ],
    sampleInput: `-----BEGIN CERTIFICATE-----
MIIC1TCCAb2gAwIBAgIUOcz/WU0laDAkDtuAmNGU4GmVt0kwDQYJKoZIhvcNAQEL
BQAwGjEYMBYGA1UEAwwPY3J5cHRvZ3JhcGh5LmlvMB4XDTIzMDkyMDE0MDgxN1oX
DTIzMTAyMTE0MDgxN1owGjEYMBYGA1UEAwwPY3J5cHRvZ3JhcGh5LmlvMIIBIjAN
BgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAo89AQYmYIa0i47P+MJXPtmv2RRHF
b1HjUQGhOPz4Q0hb7eKMadDhE5IE3QjFX7ur/hwg0zsYzVLyz0Sexp1g5GImpswr
J4VYH04vSCeabLN+YV3H2OEgpCJLlzgvU5fKPT8oIkMkvlA/B+nU7wYH6pwcWDJO
7FwP+m1ybGYeQKOlzgHRCKfgFYInLAGOJqQhSUl/BUSLHM4pJV66FbGa3AwFXYK1
V2ezNPAT0FlZtIiQIMKuGV1L20tZtdgyNAyV8T/+JVRHWQK2AFUToOypfZUCKppp
1KPnPUfEsUFHRl1EyY6wu2gAH6MKf5OZyO+0Reo7RV89aFtB6lObGxfcQQIDAQAB
oxMwETAPBgNVHQ8BAf8EBQMDAP//MA0GCSqGSIb3DQEBCwUAA4IBAQBoJXe4zOLZ
wDgYFtY/s+oakfluYLjRzIix4Tw0I1EtaDG8ivfFIwPSPOQA3CP6i5sl8mMyN9nc
irB7GC8ZlTO+FZpl+a1B1wgI07XJNDKCZ0hHC2tKQiCLl+vcagymAJuErAoHXkkZ
TBpjdBjIVTxQOEd2+zHWpSd2r531+ZteWnxcf20wJIfkPf4qWndDWMXihazfclxO
pKx84ylqy/xdIJIWxpFnd20j+wUKycD9yXZlBQggKi5mz2drZSqlurgzSjLgQu8d
zODs99oYDnekZfXLRR6Pxj3wcEJQmyqG2WtvLWDXWa7/m/N4mcjQGXp6mTkskRxj
AFpg9CCvwLY5
-----END CERTIFICATE-----`,
    sampleOutput: `[
  {
    "signature_algorithm": {
      "algorithm": "sha256_rsa",
      "parameters": null
    },
    "signature_value": "68:25:77:b8:cc:e2:d9:c0:38:18:16:d6:3f:b3:ea:1a:91:f9:6e:60:b8:d1:cc:88:b1:e1:3c:34:23:51:2d:68:31:bc:8a:f7:c5:23:03:d2:3c:e4:00:dc:23:fa:8b:9b:25:f2:63:32:37:d9:dc:8a:b0:7b:18:2f:19:95:33:be:15:9a:65:f9:ad:41:d7:08:08:d3:b5:c9:34:32:82:67:48:47:0b:6b:4a:42:20:8b:97:eb:dc:6a:0c:a6:00:9b:84:ac:0a:07:5e:49:19:4c:1a:63:74:18:c8:55:3c:50:38:47:76:fb:31:d6:a5:27:76:af:9d:f5:f9:9b:5e:5a:7c:5c:7f:6d:30:24:87:e4:3d:fe:2a:5a:77:43:58:c5:e2:85:ac:df:72:5c:4e:a4:ac:7c:e3:29:6a:cb:fc:5d:20:92:16:c6:91:67:77:6d:23:fb:05:0a:c9:c0:fd:c9:76:65:05:08:20:2a:2e:66:cf:67:6b:65:2a:a5:ba:b8:33:4a:32:e0:42:ef:1d:cc:e0:ec:f7:da:18:0e:77:a4:65:f5:cb:45:1e:8f:c6:3d:f0:70:42:50:9b:2a:86:d9:6b:6f:2d:60:d7:59:ae:ff:9b:f3:78:99:c8:d0:19:7a:7a:99:39:2c:91:1c:63:00:5a:60:f4:20:af:c0:b6:39",
    "tbs_certificate": {
      "extensions": [
        {
          "critical": true,
          "extn_id": "key_usage",
          "extn_value": [
            "crl_sign",
            "data_encipherment",
            "decipher_only",
            "digital_signature",
            "encipher_only",
            "key_agreement",
            "key_cert_sign",
            "key_encipherment",
            "non_repudiation"
          ]
        }
      ],
      "issuer": {
        "common_name": "cryptography.io"
      },
      "issuer_unique_id": null,
      "serial_number": "39:cc:ff:59:4d:25:68:30:24:0e:db:80:98:d1:94:e0:69:95:b7:49",
      "serial_number_str": "329984069978047138672494938076577194533836928841",
      "signature": {
        "algorithm": "sha256_rsa",
        "parameters": null
      },
      "subject": {
        "common_name": "cryptography.io"
      },
      "subject_public_key_info": {
        "algorithm": {
          "algorithm": "rsa",
          "parameters": null
        },
        "public_key": {
          "modulus": "a3:cf:40:41:89:98:21:ad:22:e3:b3:fe:30:95:cf:b6:6b:f6:45:11:c5:6f:51:e3:51:01:a1:38:fc:f8:43:48:5b:ed:e2:8c:69:d0:e1:13:92:04:dd:08:c5:5f:bb:ab:fe:1c:20:d3:3b:18:cd:52:f2:cf:44:9e:c6:9d:60:e4:62:26:a6:cc:2b:27:85:58:1f:4e:2f:48:27:9a:6c:b3:7e:61:5d:c7:d8:e1:20:a4:22:4b:97:38:2f:53:97:ca:3d:3f:28:22:43:24:be:50:3f:07:e9:d4:ef:06:07:ea:9c:1c:58:32:4e:ec:5c:0f:fa:6d:72:6c:66:1e:40:a3:a5:ce:01:d1:08:a7:e0:15:82:27:2c:01:8e:26:a4:21:49:49:7f:05:44:8b:1c:ce:29:25:5e:ba:15:b1:9a:dc:0c:05:5d:82:b5:57:67:b3:34:f0:13:d0:59:59:b4:88:90:20:c2:ae:19:5d:4b:db:4b:59:b5:d8:32:34:0c:95:f1:3f:fe:25:54:47:59:02:b6:00:55:13:a0:ec:a9:7d:95:02:2a:9a:69:d4:a3:e7:3d:47:c4:b1:41:47:46:5d:44:c9:8e:b0:bb:68:00:1f:a3:0a:7f:93:99:c8:ef:b4:45:ea:3b:45:5f:3d:68:5b:41:ea:53:9b:1b:17:dc:41",
          "public_exponent": 65537
        }
      },
      "subject_unique_id": null,
      "validity": {
        "not_after": 1697897297,
        "not_after_iso": "2023-10-21T14:08:17+00:00",
        "not_before": 1695218897,
        "not_before_iso": "2023-09-20T14:08:17+00:00"
      },
      "version": "v3"
    }
  }
]`,
    relatedGuides: [
      {
        href: "/guides/bash-jc-rs-jq",
        label: "Query certificate JSON with jq",
      },
    ],
  },

  x509_csr: {
    title: "CSR Decoder: Inspect a PEM Certificate Request",
    description:
      "Decode a PEM certificate signing request into JSON and inspect its subject, public key, attributes, requested extensions, and signature data.",
    eyebrow: "Certificate request decoder",
    intro:
      "A PKCS #10 certificate signing request contains the proposed subject, public key, attributes, and signature. jc-rs separates those fields into JSON so you can review what the request asks a CA to issue.",
    privacyAndValidation:
      "The request is parsed locally by WebAssembly. This is a CSR decoder, not a CSR checker: jc-rs does not verify the request signature, prove possession of the private key, approve requested names or extensions, or decide whether a CA should issue the certificate.",
    acceptedInput:
      "Paste one or more PEM blocks labelled `CERTIFICATE REQUEST`. The Windows-style `NEW CERTIFICATE REQUEST` label is accepted too. The low-level parser contains a raw-DER fallback, but the browser and current CLI public inputs are strings rather than byte buffers, so binary DER is outside this page's input promise.",
    outputDetails: [
      "Each decoded request becomes one array item with `certification_request_info`, `signature_algorithm`, and `signature`.",
      "Request information includes the v1 marker, subject distinguished name, and subject public-key algorithm and material.",
      "Attributes are returned as typed entries; extension requests and several Microsoft enrollment attributes receive structured handling when present.",
      "The signature is shown as colon-delimited hex. Its presence is visible, but its mathematical validity is not checked. Malformed blocks can result in an empty array.",
    ],
    queryLanguage: [
      "Decode a certificate signing request into structured fields",
      "View the subject and public key inside a PEM CSR",
      "Inspect requested certificate extensions without verifying the CSR",
      "Use the CSR viewer without treating its output as signature validation",
    ],
    relatedGuides: [
      {
        href: "/guides/bash-jc-rs-jq",
        label: "Query decoded request fields with jq",
      },
    ],
  },

  xml: {
    title: "XML to JSON Converter and Viewer",
    description:
      "Convert XML to JSON locally in your browser, preserving attributes, text, repeated elements, comments, and empty elements in a predictable shape.",
    eyebrow: "Private XML to JSON converter",
    intro:
      "jc-rs maps an XML document to JSON that you can inspect, copy, or pass into a shell pipeline. Its jc/xmltodict-style conventions keep attributes and mixed text distinct from child elements.",
    privacyAndValidation:
      "The local WebAssembly module parses the XML without submitting it to a service. It reports syntax errors but does not validate a DTD, XSD, or application schema. Successful conversion says nothing about whether the document satisfies a domain-specific contract.",
    acceptedInput:
      "One non-empty XML document as UTF-8 text. Elements, attributes, entity references in text, CDATA, comments, and an XML declaration are accepted. Processing instructions and document-type events are skipped, and namespace prefixes are not retained because element and attribute local names are used.",
    outputDetails: [
      "Attributes use `@name` keys, while text mixed with attributes or children uses `#text`.",
      "A text-only element becomes a JSON string; an empty element without attributes becomes `null`.",
      "Repeated sibling elements become arrays, while a single child stays a single value.",
      "Comments use `#comment`; surrounding text is trimmed, and separate text parts are joined with a space.",
    ],
    queryLanguage: [
      "Convert XML to JSON without uploading the document",
      "Use the XML viewer to inspect attributes and repeated elements as JSON",
      "See how the XML parser keeps comments and empty elements visible",
    ],
    sampleInput: `<catalog>
  <book id="b1"><title>Rust in Practice</title></book>
  <book id="b2"><title>Shell Pipelines</title></book>
</catalog>`,
    sampleOutput: `{
  "catalog": {
    "book": [
      {
        "@id": "b1",
        "title": "Rust in Practice"
      },
      {
        "@id": "b2",
        "title": "Shell Pipelines"
      }
    ]
  }
}`,
    relatedGuides: [
      {
        href: "/guides/json-vs-yaml-vs-toml",
        label: "Compare JSON, YAML, TOML, and XML",
      },
      { href: "/guides/bash-jc-rs-jq", label: "Use converted JSON with jq" },
    ],
  },

  csv: {
    title: "CSV to JSON Converter and Viewer",
    description:
      "Convert header-based CSV into an array of JSON objects locally. Quoted fields, UTF-8 BOMs, and comma, tab, or pipe delimiters are supported.",
    eyebrow: "Private CSV to JSON converter",
    intro:
      "A header-based CSV becomes one JSON object per data row. The first row supplies the property names, and every cell remains a string instead of having its type guessed from a few values.",
    privacyAndValidation:
      "Parsing stays inside the browser's WebAssembly module. jc-rs reads delimited records; it does not validate what columns mean or coerce booleans and numbers. Rows may be wider or narrower than the header, so inspect irregular source data before depending on the result.",
    acceptedInput:
      "Non-empty UTF-8 delimited text with a header row. The delimiter is inferred from the first line by comparing pipe, tab, and comma characters, with pipe and tab winning ties as defined by the parser. A UTF-8 BOM is stripped, and quoted fields may contain delimiters or newlines.",
    outputDetails: [
      "The first record supplies property names; each later record becomes one object in a JSON array.",
      "All cell values remain strings, including values that look like numbers, booleans, or dates.",
      "Rows wider than the header receive fallback names such as `col3`; rows with fewer fields simply omit the missing keys.",
      "Leading spaces before a field are normalized by the parser, while content inside a quoted field is preserved.",
    ],
    queryLanguage: [
      "Convert CSV rows to JSON objects",
      "Use the CSV viewer to inspect a file as structured JSON",
      "Parse CSV without automatic type conversion",
    ],
    sampleInput: `name,role,active
Ada,admin,true
Lin,reader,false`,
    sampleOutput: `[
  {
    "active": "true",
    "name": "Ada",
    "role": "admin"
  },
  {
    "active": "false",
    "name": "Lin",
    "role": "reader"
  }
]`,
    relatedGuides: [
      { href: "/guides/bash-jc-rs-jq", label: "Filter converted rows with jq" },
    ],
  },

  yaml: {
    title: "YAML to JSON Converter and Viewer",
    description:
      "Convert single- or multi-document YAML to structured JSON in your browser, with YAML scalar, sequence, and mapping types preserved where JSON allows.",
    eyebrow: "Private YAML to JSON converter",
    intro:
      "jc-rs converts ordinary YAML configuration and `---`-separated document streams into the same predictable JSON shape: an array with one entry per document.",
    privacyAndValidation:
      "Conversion happens locally in WebAssembly. The parser checks YAML syntax, not a Kubernetes, CI, or application schema. YAML parsing rules determine scalar types, so quote identifiers and version-like values when they must remain strings.",
    acceptedInput:
      "One or more non-empty YAML documents as UTF-8 text. Mappings, sequences, scalars, nested values, and `---` document separators are accepted. Each document is converted through JSON-compatible values.",
    outputDetails: [
      "The result is always an array, even when the input contains a single YAML document.",
      "A top-level mapping becomes one object; a top-level sequence or scalar is wrapped in an object under `value`.",
      "YAML strings, numbers, booleans, nulls, arrays, and mappings become their JSON counterparts.",
      "A multi-document stream produces one array item per document in source order.",
    ],
    queryLanguage: [
      "Convert YAML configuration to JSON",
      "Use the YAML viewer to inspect a multi-document stream as JSON",
      "See how the YAML parser maps values to JSON types",
    ],
    sampleInput: `service: api
replicas: 3
enabled: true
tags:
  - web
  - internal`,
    sampleOutput: `[
  {
    "enabled": true,
    "replicas": 3,
    "service": "api",
    "tags": [
      "web",
      "internal"
    ]
  }
]`,
    relatedGuides: [
      {
        href: "/guides/json-vs-yaml-vs-toml",
        label: "Choose between JSON, YAML, and TOML",
      },
      { href: "/guides/bash-jc-rs-jq", label: "Use YAML-derived JSON with jq" },
    ],
  },

  url: {
    title: "URL Parser: Break a URL Into Its Parts",
    description:
      "Parse an absolute URL into scheme, credentials, host, port, path, filename, query values, and fragment, with encoded and decoded component views.",
    eyebrow: "Private URL component parser",
    intro:
      "jc-rs separates a complete URL into its scheme, credentials, host, port, path, query, and fragment. It also derives filename and path fields and retains every value attached to a repeated query key.",
    privacyAndValidation:
      "The parser runs locally. It never fetches the URL, resolves its host, or decides whether the destination is reachable or safe. This page parses URL structure; it is not a phishing check, allow-list check, or general-purpose percent encoder or decoder.",
    acceptedInput:
      "One absolute URL with a scheme, such as `https://`, `ftp://`, or another scheme accepted by the URL parser. Plain relative paths are rejected. Angle brackets and a leading `URL:` wrapper are removed. Percent escapes in query keys and values are decoded, but `+` is not treated as a space.",
    outputDetails: [
      "Top-level fields include `scheme`, `netloc`, `hostname`, `port`, `username`, `password`, `path`, `query`, and `fragment`.",
      "Path helpers include `parent`, `filename`, `stem`, `extension`, and `path_list`; duplicate path slashes are collapsed.",
      "`query_obj` maps each percent-decoded key to an array, preserving repeated values instead of silently keeping only one.",
      "Nested `encoded` and `decoded` objects show parser-produced component views. They do not turn arbitrary free-form text into or out of percent encoding.",
    ],
    queryLanguage: [
      "Parse a URL into host, path, query, and fragment",
      "Read repeated query-string values without fetching the URL",
      "Inspect the parser's encoded and decoded component views",
    ],
    sampleInput: "https://example.com/report.json?tag=rust&tag=cli#results",
    sampleOutput: `{
  "decoded": {
    "extension": "json",
    "filename": "report.json",
    "fragment": "results",
    "hostname": "example.com",
    "netloc": "example.com",
    "parent": "/",
    "password": null,
    "path": "/report.json",
    "path_list": [
      "report.json"
    ],
    "port": null,
    "query": "tag=rust&tag=cli",
    "scheme": "https",
    "stem": "report",
    "url": "https://example.com/report.json?tag=rust&tag=cli#results",
    "username": null
  },
  "encoded": {
    "extension": "json",
    "filename": "report.json",
    "fragment": "results",
    "hostname": "example.com",
    "netloc": "example.com",
    "parent": "/",
    "password": null,
    "path": "/report%2Ejson",
    "path_list": [
      "report.json"
    ],
    "port": null,
    "query": "tag%3Drust%26tag%3Dcli",
    "scheme": "https",
    "stem": "report",
    "url": "https%3A%2F%2Fexample%2Ecom%2Freport%2Ejson%3Ftag%3Drust%26tag%3Dcli%23results",
    "username": null
  },
  "extension": "json",
  "filename": "report.json",
  "fragment": "results",
  "hostname": "example.com",
  "netloc": "example.com",
  "parent": "/",
  "password": null,
  "path": "/report.json",
  "path_list": [
    "report.json"
  ],
  "port": null,
  "query": "tag=rust&tag=cli",
  "query_obj": {
    "tag": [
      "rust",
      "cli"
    ]
  },
  "scheme": "https",
  "stem": "report",
  "url": "https://example.com/report.json?tag=rust&tag=cli#results",
  "username": null
}`,
    relatedGuides: [
      {
        href: "/guides/bash-jc-rs-jq",
        label: "Use parsed URL fields in a shell pipeline",
      },
    ],
  },

  toml: {
    title: "TOML to JSON Converter",
    description:
      "Convert a TOML document to JSON locally, preserving strings, numbers, booleans, arrays, nested tables, and explicit datetime representations.",
    eyebrow: "Private TOML to JSON converter",
    intro:
      "jc-rs converts a TOML configuration file to JSON without flattening tables or guessing scalar types. The resulting object is ready for inspection or for tools that already consume JSON.",
    privacyAndValidation:
      "The document is parsed inside the browser. jc-rs reports TOML syntax errors, but it cannot decide whether the settings are valid for Cargo, an application, or another consumer. This is structural conversion, not configuration validation.",
    acceptedInput:
      "One non-empty TOML document whose top level is a table. Standard strings, integers, floats, booleans, datetimes, arrays, inline tables, named tables, and arrays of tables are handled by the TOML parser.",
    outputDetails: [
      "Strings, integers, floats, booleans, arrays, and nested tables retain their corresponding JSON shapes.",
      "A datetime stored under a table key produces a Unix timestamp plus a sibling key ending in `_iso`.",
      "Datetime values inside arrays remain ISO strings, matching jc's established output schema.",
      "Invalid TOML stops with a parse error instead of returning a partial object.",
    ],
    queryLanguage: [
      "Convert a TOML configuration file to JSON",
      "Inspect nested TOML tables as JSON objects",
      "Check how TOML datetime values appear in jc-rs output",
    ],
    sampleInput: `title = "jc-rs"
enabled = true

[server]
host = "127.0.0.1"
port = 8080`,
    sampleOutput: `{
  "enabled": true,
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "title": "jc-rs"
}`,
    relatedGuides: [
      {
        href: "/guides/json-vs-yaml-vs-toml",
        label: "Choose between JSON, YAML, and TOML",
      },
      {
        href: "/guides/bash-jc-rs-jq",
        label: "Query TOML-derived JSON with jq",
      },
    ],
  },

  tsv: {
    title: "TSV to JSON Converter and Viewer",
    description:
      "Convert a header-based tab-separated table into JSON objects locally, including quoted fields and records that span more than one line.",
    eyebrow: "Private TSV to JSON converter",
    intro:
      "Header-based tab-separated data becomes one JSON object per row. jc-rs keeps every cell as text, so identifiers and zero-padded values are not silently converted to numbers.",
    privacyAndValidation:
      "Parsing runs locally in WebAssembly. jc-rs recognizes tab-delimited records but does not infer types or validate the meaning of each column. Uneven rows are represented as received rather than rejected as schema violations.",
    acceptedInput:
      "Non-empty UTF-8 text with tab characters between fields and a header in the first record. A UTF-8 BOM is stripped. Quoted cells can contain tabs or newlines; blank lines between records are ignored.",
    outputDetails: [
      "Every data record becomes an object in a JSON array, keyed by the first record's column names.",
      "Every cell remains a JSON string, including numeric-looking and boolean-looking values.",
      "A row longer than the header receives fallback keys such as `c2`; a shorter row contains only the fields actually present.",
      "Quoted multiline cells are accumulated until the record closes instead of being split into separate rows.",
    ],
    queryLanguage: [
      "Convert a TSV file to an array of JSON objects",
      "Inspect tab-separated values without changing their types",
      "Parse quoted and multiline TSV records",
    ],
    sampleInput: "name\trole\nAda\tadmin\nLin\treader",
    sampleOutput: `[
  {
    "name": "Ada",
    "role": "admin"
  },
  {
    "name": "Lin",
    "role": "reader"
  }
]`,
    relatedGuides: [
      { href: "/guides/bash-jc-rs-jq", label: "Filter tabular JSON with jq" },
    ],
  },

  plist: {
    title: "Plist to JSON Converter and Viewer",
    description:
      "Read XML or OpenStep property-list text as JSON locally, preserving nested values and making plist dates and byte data easier to inspect.",
    eyebrow: "Private text plist viewer",
    intro:
      "jc-rs opens textual Apple property lists without a platform-specific viewer. Dictionaries, arrays, scalars, dates, byte data, and UIDs become JSON while retaining the document's nesting.",
    privacyAndValidation:
      "The local WebAssembly module parses the property list. It checks the serialization it can read, not the preference schema of a particular macOS or iOS application, and it does not assess signatures, entitlements, or whether editing a value would be safe.",
    acceptedInput:
      "A non-empty XML plist or OpenStep/NeXTSTEP plist as UTF-8 text. The underlying parser also supports binary plist bytes, but the browser and current CLI interfaces accept strings, so this page does not claim reliable binary plist upload or conversion.",
    outputDetails: [
      "Dictionaries and arrays remain JSON objects and arrays; strings, integers, real numbers, booleans, and UIDs retain appropriate JSON scalar forms.",
      "Byte-data values become lowercase, colon-delimited hex strings.",
      "Dictionary dates produce an epoch value plus an `_iso` sibling; dates inside arrays are represented by epoch values.",
      "A non-dictionary root is wrapped under `value` so the parser can still return a JSON object.",
    ],
    queryLanguage: [
      "Inspect an XML property list as JSON",
      "Convert a text plist without uploading the file",
      "Review plist dates, data values, dictionaries, and arrays",
    ],
    sampleInput: `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Name</key><string>jc-rs</string>
  <key>Enabled</key><true/>
  <key>Count</key><integer>3</integer>
</dict>
</plist>`,
    sampleOutput: `{
  "Count": 3,
  "Enabled": true,
  "Name": "jc-rs"
}`,
    relatedGuides: [
      {
        href: "/guides/bash-jc-rs-jq",
        label: "Query property-list JSON with jq",
      },
    ],
  },

  asciitable: {
    title: "ASCII Table to JSON Converter",
    description:
      "Convert a single-line ASCII, Unicode, Markdown, or fixed-width table into JSON records while preserving cell text and empty values.",
    eyebrow: "Structured table to JSON",
    intro:
      "jc-rs reads the header and column positions in an aligned terminal table, ignores common border and row-separator styles, and emits one object for each single-line data row.",
    privacyAndValidation:
      "The table is parsed locally. The parser needs visible structure: prose, wrapped rows, ambiguous spacing, or duplicate headers can produce misleading columns without triggering an error. Check the preview before automating an unfamiliar source.",
    acceptedInput:
      "A table with one header row and one physical line per data row. Plain fixed-width tables, pipe-based Markdown tables, and common ASCII or box-drawing borders are accepted; row separators and ANSI terminal color sequences are ignored. In plain tables, column headers need clear spacing and should be unique.",
    outputDetails: [
      "Each data row becomes one object in an array; separator lines do not become records.",
      "Header labels are normalized to lowercase keys with snake-case-style separators.",
      "Cell values remain strings, and an empty cell becomes `null`.",
      "Use the multiline-table parser when one logical row spans several physical lines.",
    ],
    queryLanguage: [
      "Convert an ASCII table to JSON records",
      "Parse a Markdown or Unicode table from terminal output",
      "Convert a fixed-width table without treating prose as tabular data",
    ],
    sampleInput: `NAME      STATUS   COUNT
api       ready    3
worker    pending  1`,
    sampleOutput: `[
  {
    "count": "3",
    "name": "api",
    "status": "ready"
  },
  {
    "count": "1",
    "name": "worker",
    "status": "pending"
  }
]`,
    relatedGuides: [
      {
        href: "/guides/ascii-table-to-json",
        label: "Convert terminal tables safely",
      },
      {
        href: "/guides/parsing-command-output-reliably",
        label: "Understand where command-output parsing breaks",
      },
    ],
  },

  asciitable_m: {
    title: "Multiline ASCII Table to JSON Converter",
    description:
      "Convert bordered ASCII or Unicode tables with wrapped cells and multiline headers into JSON, joining each logical cell with explicit newlines.",
    eyebrow: "Multiline table to JSON",
    intro:
      "This parser handles pretty-printed tables in which one logical row wraps across several terminal lines. Row separators define record boundaries, and consecutive lines from the same cell stay together.",
    privacyAndValidation:
      "Parsing stays in the browser. Visible borders and row separators are part of the grammar, so simple and Markdown tables are intentionally rejected. Free-form text and tables that use separator characters inside cell data are not safe inputs.",
    acceptedInput:
      "A `pretty` table drawn with ASCII or Unicode column borders, with a header area and a recognized separator between logical rows. Headers and data may span several lines. Simple whitespace-aligned and Markdown tables are rejected with a direction to use `asciitable` instead.",
    outputDetails: [
      "Each bordered row group becomes one JSON object, regardless of how many physical lines its cells occupy.",
      "Lines from the same cell are trimmed and joined with `\\n`, preserving the visible wrap in a machine-readable string.",
      "Multiline header parts are combined with underscores and normalized to lowercase keys.",
      "Cell values remain strings; cells with no content become `null`.",
    ],
    queryLanguage: [
      "Convert a multiline terminal table to JSON",
      "Parse wrapped cells in an ASCII or Unicode table",
      "Keep each multiline table row together in one JSON object",
    ],
    sampleInput: `+----------+---------+
| Name     | Notes   |
+==========+=========+
| api      | ready   |
|          | primary |
+----------+---------+
| worker   | pending |
|          | batch   |
+----------+---------+`,
    sampleOutput: `[
  {
    "name": "api",
    "notes": "ready\\nprimary"
  },
  {
    "name": "worker",
    "notes": "pending\\nbatch"
  }
]`,
    relatedGuides: [
      {
        href: "/guides/ascii-table-to-json",
        label: "Choose the right table parser",
      },
      {
        href: "/guides/parsing-command-output-reliably",
        label: "Recognize fragile command-output layouts",
      },
    ],
  },
};

export function getParserSeo(name: string): ParserSeoContent | undefined {
  return Object.hasOwn(parserSeo, name)
    ? parserSeo[name as ParserSeoName]
    : undefined;
}
