import next from "eslint-config-next";

const config = [
  ...next,
  { ignores: [".next/**", "node_modules/**", "public/wasm/**", "src/data/**"] },
];

export default config;
