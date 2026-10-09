import test from "node:test";
import assert from "node:assert/strict";
import { isValidModelId, normalizeModelId } from "./model_id.mjs";

test("accepts valid repository IDs", () => {
  assert.equal(normalizeModelId("Qwen/Qwen2.5-Coder-32B-Instruct"), "Qwen/Qwen2.5-Coder-32B-Instruct");
  assert.equal(isValidModelId("zai-org/GLM-5.3"), true);
});
test("repairs the exact legacy concatenation reported by MINUX Agent", () => {
  assert.equal(normalizeModelId("zai-org/GLM-5.3Qwen/Qwen2.5-Coder-32B-Instruct"), "zai-org/GLM-5.3");
});
test("rejects malformed identifiers without recovery", () => {
  assert.equal(isValidModelId("owner/model/second"), false);
  assert.throws(() => normalizeModelId("owner/model/second"));
});
