# Service Layer

## Model
1. find node by `node_id`
2. request `ServiceRecord` by exact `app_id`
3. open application session

## Rule
No global service enumeration in MVP.

## Current Milestone 8 baseline
- `GetServiceRecord` is exact-by-`app_id` only.
- Service registration stays local, bounded, and signature-verified before the
  registry stores a signed `ServiceRecord`.
- `OpenAppSession` binds to the advertised `ServiceRecord.reachability_ref`
  instead of bypassing the existing node-reachability flow.
- Local service access policy is allow-or-deny only for the current baseline.
