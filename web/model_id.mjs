/** JavaScript model-ID guard used in the toolchain tests. */
export const suggestedModelIds = Object.freeze([
  "Qwen/Qwen2.5-Coder-32B-Instruct",
  "Qwen/Qwen3-8B",
  "deepseek-ai/DeepSeek-R1",
  "meta-llama/Llama-3.3-70B-Instruct",
  "mistralai/Mistral-7B-Instruct-v0.3",
]);
export function isValidModelId(value) {
  if (value.length > 256 || value.trim() !== value || /\s/.test(value)) return false;
  const parts = value.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) return false;
  return parts.every((part) => /^[A-Za-z0-9._-]+$/.test(part)
    && !part.startsWith(".") && !part.endsWith(".") && !part.includes(".."));
}
export function normalizeModelId(input) {
  const value = input.trim().replace(/^['"`]|['"`]$/g, "");
  if (isValidModelId(value)) return value;
  for (const candidate of suggestedModelIds) {
    if (value.endsWith(candidate)) {
      const prefix = value.slice(0, value.length - candidate.length).trim();
      if (isValidModelId(prefix)) return prefix;
    }
  }
  const matches = suggestedModelIds.filter((candidate) => value.includes(candidate));
  if (matches.length === 1) return matches[0];
  throw new Error("Model ID must use owner/model format; choose a model from the list.");
}
