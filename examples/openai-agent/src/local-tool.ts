export type PolicyLookupResult =
  | { result: "ok"; summary: string }
  | { result: "error"; summary: string; errorCode?: string };

const knownPolicyKeys = new Set(["synthetic.shipping.standard"]);

export function lookupPolicy(policyKey: string): PolicyLookupResult {
  if (!knownPolicyKeys.has(policyKey)) {
    return {
      result: "error",
      summary: "No matching synthetic policy fixture",
      errorCode: "LOCAL_POLICY_NOT_FOUND",
    };
  }
  return { result: "ok", summary: "Synthetic policy fixture matched" };
}
