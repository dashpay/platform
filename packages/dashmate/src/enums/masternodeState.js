// according doc https://dashcore.readme.io/docs/core-api-ref-remote-procedure-calls-dash#masternode-status
const MasternodeStateEnum = {
  WAITING_FOR_PROTX: "WAITING_FOR_PROTX",
  POSE_BANNED: "POSE_BANNED",
  REMOVED: "REMOVED",
  OPERATOR_KEY_CHANGED: "OPERATOR_KEY_CHANGED",
  PROTX_IP_CHANGED: "PROTX_IP_CHANGED",
  READY: "READY",
  ERROR: "ERROR",
  UNKNOWN: "UNKNOWN"
}

module.exports = MasternodeStateEnum
