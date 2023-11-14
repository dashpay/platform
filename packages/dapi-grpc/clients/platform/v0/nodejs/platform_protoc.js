// source: platform.proto
/**
 * @fileoverview
 * @enhanceable
 * @suppress {missingRequire} reports error on implicit type usages.
 * @suppress {messageConventions} JS Compiler reports an error if a variable or
 *     field starts with 'MSG_' and isn't a translatable message.
 * @public
 */
// GENERATED CODE -- DO NOT EDIT!
/* eslint-disable */
// @ts-nocheck

var jspb = require('google-protobuf');
var goog = jspb;
const proto = {};

var google_protobuf_wrappers_pb = require('google-protobuf/google/protobuf/wrappers_pb.js');
goog.object.extend(proto, google_protobuf_wrappers_pb);
var google_protobuf_struct_pb = require('google-protobuf/google/protobuf/struct_pb.js');
goog.object.extend(proto, google_protobuf_struct_pb);
var google_protobuf_timestamp_pb = require('google-protobuf/google/protobuf/timestamp_pb.js');
goog.object.extend(proto, google_protobuf_timestamp_pb);
goog.exportSymbol('proto.org.dash.platform.dapi.v0.AllKeys', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.StartCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.Type', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.KeyRequestType', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.KeyRequestType.RequestCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.Proof', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.ResponseMetadata', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.SearchKey', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.SecurityLevelMap', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.SpecificKeys', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.VersionCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.ResultCase', null, { proto });
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.Proof = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.Proof, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.Proof.displayName = 'proto.org.dash.platform.dapi.v0.Proof';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.ResponseMetadata, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.ResponseMetadata.displayName = 'proto.org.dash.platform.dapi.v0.ResponseMetadata';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.displayName = 'proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.displayName = 'proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.displayName = 'proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.KeyRequestType = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.KeyRequestType.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.KeyRequestType, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.KeyRequestType.displayName = 'proto.org.dash.platform.dapi.v0.KeyRequestType';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.AllKeys = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.AllKeys, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.AllKeys.displayName = 'proto.org.dash.platform.dapi.v0.AllKeys';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.SpecificKeys = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.SpecificKeys.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.SpecificKeys, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.SpecificKeys.displayName = 'proto.org.dash.platform.dapi.v0.SpecificKeys';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.SearchKey = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.SearchKey, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.SearchKey.displayName = 'proto.org.dash.platform.dapi.v0.SearchKey';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.SecurityLevelMap, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.SecurityLevelMap.displayName = 'proto.org.dash.platform.dapi.v0.SecurityLevelMap';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProofsRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProofsResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractsRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractsRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractsRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractsRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractsResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractsResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDocumentsRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDocumentsRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetDocumentsRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDocumentsResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetDocumentsResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.displayName = 'proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.displayName = 'proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.displayName = 'proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.displayName = 'proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.displayName = 'proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.displayName = 'proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.displayName = 'proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0 = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.displayName = 'proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.displayName = 'proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.displayName = 'proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo';
}



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.Proof.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.Proof} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.Proof.toObject = function(includeInstance, msg) {
  var f, obj = {
    grovedbProof: msg.getGrovedbProof_asB64(),
    quorumHash: msg.getQuorumHash_asB64(),
    signature: msg.getSignature_asB64(),
    round: jspb.Message.getFieldWithDefault(msg, 4, 0),
    blockIdHash: msg.getBlockIdHash_asB64(),
    quorumType: jspb.Message.getFieldWithDefault(msg, 6, 0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.Proof.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.Proof;
  return proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.Proof} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setGrovedbProof(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setQuorumHash(value);
      break;
    case 3:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setSignature(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setRound(value);
      break;
    case 5:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setBlockIdHash(value);
      break;
    case 6:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setQuorumType(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.Proof} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getGrovedbProof_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getQuorumHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
  f = message.getSignature_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      3,
      f
    );
  }
  f = message.getRound();
  if (f !== 0) {
    writer.writeUint32(
      4,
      f
    );
  }
  f = message.getBlockIdHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      5,
      f
    );
  }
  f = message.getQuorumType();
  if (f !== 0) {
    writer.writeUint32(
      6,
      f
    );
  }
};


/**
 * optional bytes grovedb_proof = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getGrovedbProof = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes grovedb_proof = 1;
 * This is a type-conversion wrapper around `getGrovedbProof()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getGrovedbProof_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getGrovedbProof()));
};


/**
 * optional bytes grovedb_proof = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getGrovedbProof()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getGrovedbProof_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getGrovedbProof()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.Proof} returns this
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.setGrovedbProof = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bytes quorum_hash = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getQuorumHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes quorum_hash = 2;
 * This is a type-conversion wrapper around `getQuorumHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getQuorumHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getQuorumHash()));
};


/**
 * optional bytes quorum_hash = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getQuorumHash()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getQuorumHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getQuorumHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.Proof} returns this
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.setQuorumHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};


/**
 * optional bytes signature = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getSignature = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * optional bytes signature = 3;
 * This is a type-conversion wrapper around `getSignature()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getSignature_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getSignature()));
};


/**
 * optional bytes signature = 3;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getSignature()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getSignature_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getSignature()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.Proof} returns this
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.setSignature = function(value) {
  return jspb.Message.setProto3BytesField(this, 3, value);
};


/**
 * optional uint32 round = 4;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getRound = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.Proof} returns this
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.setRound = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional bytes block_id_hash = 5;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getBlockIdHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 5, ""));
};


/**
 * optional bytes block_id_hash = 5;
 * This is a type-conversion wrapper around `getBlockIdHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getBlockIdHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getBlockIdHash()));
};


/**
 * optional bytes block_id_hash = 5;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getBlockIdHash()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getBlockIdHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getBlockIdHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.Proof} returns this
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.setBlockIdHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 5, value);
};


/**
 * optional uint32 quorum_type = 6;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.getQuorumType = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 6, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.Proof} returns this
 */
proto.org.dash.platform.dapi.v0.Proof.prototype.setQuorumType = function(value) {
  return jspb.Message.setProto3IntField(this, 6, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.ResponseMetadata} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject = function(includeInstance, msg) {
  var f, obj = {
    height: jspb.Message.getFieldWithDefault(msg, 1, 0),
    coreChainLockedHeight: jspb.Message.getFieldWithDefault(msg, 2, 0),
    epoch: jspb.Message.getFieldWithDefault(msg, 3, 0),
    timeMs: jspb.Message.getFieldWithDefault(msg, 4, 0),
    protocolVersion: jspb.Message.getFieldWithDefault(msg, 5, 0),
    chainId: jspb.Message.getFieldWithDefault(msg, 6, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
  return proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.ResponseMetadata} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setHeight(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setCoreChainLockedHeight(value);
      break;
    case 3:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setEpoch(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setTimeMs(value);
      break;
    case 5:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setProtocolVersion(value);
      break;
    case 6:
      var value = /** @type {string} */ (reader.readString());
      msg.setChainId(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.ResponseMetadata} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getHeight();
  if (f !== 0) {
    writer.writeUint64(
      1,
      f
    );
  }
  f = message.getCoreChainLockedHeight();
  if (f !== 0) {
    writer.writeUint32(
      2,
      f
    );
  }
  f = message.getEpoch();
  if (f !== 0) {
    writer.writeUint32(
      3,
      f
    );
  }
  f = message.getTimeMs();
  if (f !== 0) {
    writer.writeUint64(
      4,
      f
    );
  }
  f = message.getProtocolVersion();
  if (f !== 0) {
    writer.writeUint32(
      5,
      f
    );
  }
  f = message.getChainId();
  if (f.length > 0) {
    writer.writeString(
      6,
      f
    );
  }
};


/**
 * optional uint64 height = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional uint32 core_chain_locked_height = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getCoreChainLockedHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setCoreChainLockedHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional uint32 epoch = 3;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getEpoch = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 3, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setEpoch = function(value) {
  return jspb.Message.setProto3IntField(this, 3, value);
};


/**
 * optional uint64 time_ms = 4;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getTimeMs = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setTimeMs = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional uint32 protocol_version = 5;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getProtocolVersion = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 5, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setProtocolVersion = function(value) {
  return jspb.Message.setProto3IntField(this, 5, value);
};


/**
 * optional string chain_id = 6;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getChainId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 6, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setChainId = function(value) {
  return jspb.Message.setProto3StringField(this, 6, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.toObject = function(includeInstance, msg) {
  var f, obj = {
    code: jspb.Message.getFieldWithDefault(msg, 1, 0),
    message: jspb.Message.getFieldWithDefault(msg, 2, ""),
    data: msg.getData_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError;
  return proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setCode(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setMessage(value);
      break;
    case 3:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setData(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getCode();
  if (f !== 0) {
    writer.writeUint32(
      1,
      f
    );
  }
  f = message.getMessage();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getData_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      3,
      f
    );
  }
};


/**
 * optional uint32 code = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.getCode = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} returns this
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.setCode = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional string message = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.getMessage = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} returns this
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.setMessage = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional bytes data = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.getData = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * optional bytes data = 3;
 * This is a type-conversion wrapper around `getData()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.getData_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getData()));
};


/**
 * optional bytes data = 3;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getData()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.getData_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getData()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} returns this
 */
proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.prototype.setData = function(value) {
  return jspb.Message.setProto3BytesField(this, 3, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    stateTransition: msg.getStateTransition_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest;
  return proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateTransition(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getStateTransition_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
};


/**
 * optional bytes state_transition = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.prototype.getStateTransition = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes state_transition = 1;
 * This is a type-conversion wrapper around `getStateTransition()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.prototype.getStateTransition_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateTransition()));
};


/**
 * optional bytes state_transition = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateTransition()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.prototype.getStateTransition_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateTransition()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} returns this
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.prototype.setStateTransition = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.toObject = function(includeInstance, msg) {
  var f, obj = {

  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse;
  return proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentityRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    id: msg.getId_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setId(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * optional bytes id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.getId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getId()));
};


/**
 * optional bytes id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetIdentityRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityRequest.GetIdentityRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    id: msg.getId_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setId(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * optional bytes id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.getId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getId()));
};


/**
 * optional bytes id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetIdentityBalanceRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.GetIdentityBalanceRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    id: msg.getId_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setId(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * optional bytes id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.getId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getId()));
};


/**
 * optional bytes id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetIdentityBalanceAndRevisionRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentityResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITY: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    identity: msg.getIdentity_asB64(),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setIdentity(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = /** @type {!(string|Uint8Array)} */ (jspb.Message.getField(message, 1));
  if (f != null) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes identity = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.getIdentity = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity = 1;
 * This is a type-conversion wrapper around `getIdentity()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.getIdentity_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getIdentity()));
};


/**
 * optional bytes identity = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdentity()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.getIdentity_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentity()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.setIdentity = function(value) {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.clearIdentity = function() {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.hasIdentity = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetIdentityResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityResponse.GetIdentityResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    idsList: msg.getIdsList_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addIds(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdsList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * repeated bytes ids = 1;
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.getIdsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes ids = 1;
 * This is a type-conversion wrapper around `getIdsList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.getIdsList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getIdsList()));
};


/**
 * repeated bytes ids = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdsList()`
 * @return {!Array<!Uint8Array>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.getIdsList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getIdsList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.setIdsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.addIds = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.clearIdsList = function() {
  return this.setIdsList([]);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetIdentitiesRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.GetIdentitiesRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.toObject = function(includeInstance, msg) {
  var f, obj = {
    value: msg.getValue_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setValue(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getValue_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
};


/**
 * optional bytes value = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.prototype.getValue = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes value = 1;
 * This is a type-conversion wrapper around `getValue()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.prototype.getValue_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getValue()));
};


/**
 * optional bytes value = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getValue()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.prototype.getValue_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getValue()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.prototype.setValue = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.toObject = function(includeInstance, msg) {
  var f, obj = {
    key: msg.getKey_asB64(),
    value: (f = msg.getValue()) && proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setKey(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.deserializeBinaryFromReader);
      msg.setValue(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getKey_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getValue();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes key = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.getKey = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes key = 1;
 * This is a type-conversion wrapper around `getKey()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.getKey_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getKey()));
};


/**
 * optional bytes key = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getKey()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.getKey_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getKey()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.setKey = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional IdentityValue value = 2;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.getValue = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.setValue = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.clearValue = function() {
  return this.setValue(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.prototype.hasValue = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.toObject = function(includeInstance, msg) {
  var f, obj = {
    identityEntriesList: jspb.Message.toObjectList(msg.getIdentityEntriesList(),
    proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.deserializeBinaryFromReader);
      msg.addIdentityEntries(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentityEntriesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry.serializeBinaryToWriter
    );
  }
};


/**
 * repeated IdentityEntry identity_entries = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.prototype.getIdentityEntriesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.prototype.setIdentityEntriesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.prototype.addIdentityEntries = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.prototype.clearIdentityEntriesList = function() {
  return this.setIdentityEntriesList([]);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITIES: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    identities: (f = msg.getIdentities()) && proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.deserializeBinaryFromReader);
      msg.setIdentities(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentities();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional Identities identities = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.getIdentities = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.setIdentities = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.clearIdentities = function() {
  return this.setIdentities(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.hasIdentities = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetIdentitiesResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.GetIdentitiesResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  BALANCE: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    balance: jspb.Message.getFieldWithDefault(msg, 1, 0),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setBalance(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = /** @type {number} */ (jspb.Message.getField(message, 1));
  if (f != null) {
    writer.writeUint64(
      1,
      f
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional uint64 balance = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.getBalance = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.setBalance = function(value) {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.clearBalance = function() {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.hasBalance = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetIdentityBalanceResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.GetIdentityBalanceResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  BALANCE_AND_REVISION: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    balanceAndRevision: (f = msg.getBalanceAndRevision()) && proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.deserializeBinaryFromReader);
      msg.setBalanceAndRevision(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getBalanceAndRevision();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.toObject = function(includeInstance, msg) {
  var f, obj = {
    balance: jspb.Message.getFieldWithDefault(msg, 1, 0),
    revision: jspb.Message.getFieldWithDefault(msg, 2, 0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setBalance(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setRevision(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getBalance();
  if (f !== 0) {
    writer.writeUint64(
      1,
      f
    );
  }
  f = message.getRevision();
  if (f !== 0) {
    writer.writeUint64(
      2,
      f
    );
  }
};


/**
 * optional uint64 balance = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.prototype.getBalance = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.prototype.setBalance = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional uint64 revision = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.prototype.getRevision = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.prototype.setRevision = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional BalanceAndRevision balance_and_revision = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.getBalanceAndRevision = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.setBalanceAndRevision = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.clearBalanceAndRevision = function() {
  return this.setBalanceAndRevision(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.hasBalanceAndRevision = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetIdentityBalanceAndRevisionResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.oneofGroups_ = [[1,2,3]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.RequestCase = {
  REQUEST_NOT_SET: 0,
  ALL_KEYS: 1,
  SPECIFIC_KEYS: 2,
  SEARCH_KEY: 3
};

/**
 * @return {proto.org.dash.platform.dapi.v0.KeyRequestType.RequestCase}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.getRequestCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.KeyRequestType.RequestCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.KeyRequestType.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.KeyRequestType.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.KeyRequestType} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.toObject = function(includeInstance, msg) {
  var f, obj = {
    allKeys: (f = msg.getAllKeys()) && proto.org.dash.platform.dapi.v0.AllKeys.toObject(includeInstance, f),
    specificKeys: (f = msg.getSpecificKeys()) && proto.org.dash.platform.dapi.v0.SpecificKeys.toObject(includeInstance, f),
    searchKey: (f = msg.getSearchKey()) && proto.org.dash.platform.dapi.v0.SearchKey.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.KeyRequestType;
  return proto.org.dash.platform.dapi.v0.KeyRequestType.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.KeyRequestType} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.AllKeys;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.AllKeys.deserializeBinaryFromReader);
      msg.setAllKeys(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.SpecificKeys;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.SpecificKeys.deserializeBinaryFromReader);
      msg.setSpecificKeys(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.SearchKey;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.SearchKey.deserializeBinaryFromReader);
      msg.setSearchKey(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.KeyRequestType.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.KeyRequestType} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getAllKeys();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.AllKeys.serializeBinaryToWriter
    );
  }
  f = message.getSpecificKeys();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.SpecificKeys.serializeBinaryToWriter
    );
  }
  f = message.getSearchKey();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.SearchKey.serializeBinaryToWriter
    );
  }
};


/**
 * optional AllKeys all_keys = 1;
 * @return {?proto.org.dash.platform.dapi.v0.AllKeys}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.getAllKeys = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.AllKeys} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.AllKeys, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.AllKeys|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType} returns this
*/
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.setAllKeys = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.KeyRequestType.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType} returns this
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.clearAllKeys = function() {
  return this.setAllKeys(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.hasAllKeys = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional SpecificKeys specific_keys = 2;
 * @return {?proto.org.dash.platform.dapi.v0.SpecificKeys}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.getSpecificKeys = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.SpecificKeys} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.SpecificKeys, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.SpecificKeys|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType} returns this
*/
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.setSpecificKeys = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.KeyRequestType.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType} returns this
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.clearSpecificKeys = function() {
  return this.setSpecificKeys(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.hasSpecificKeys = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional SearchKey search_key = 3;
 * @return {?proto.org.dash.platform.dapi.v0.SearchKey}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.getSearchKey = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.SearchKey} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.SearchKey, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.SearchKey|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType} returns this
*/
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.setSearchKey = function(value) {
  return jspb.Message.setOneofWrapperField(this, 3, proto.org.dash.platform.dapi.v0.KeyRequestType.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.KeyRequestType} returns this
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.clearSearchKey = function() {
  return this.setSearchKey(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.KeyRequestType.prototype.hasSearchKey = function() {
  return jspb.Message.getField(this, 3) != null;
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.AllKeys.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.AllKeys.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.AllKeys} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.AllKeys.toObject = function(includeInstance, msg) {
  var f, obj = {

  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.AllKeys}
 */
proto.org.dash.platform.dapi.v0.AllKeys.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.AllKeys;
  return proto.org.dash.platform.dapi.v0.AllKeys.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.AllKeys} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.AllKeys}
 */
proto.org.dash.platform.dapi.v0.AllKeys.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.AllKeys.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.AllKeys.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.AllKeys} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.AllKeys.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.SpecificKeys.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.SpecificKeys} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.toObject = function(includeInstance, msg) {
  var f, obj = {
    keyIdsList: (f = jspb.Message.getRepeatedField(msg, 1)) == null ? undefined : f
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.SpecificKeys}
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.SpecificKeys;
  return proto.org.dash.platform.dapi.v0.SpecificKeys.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.SpecificKeys} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.SpecificKeys}
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var values = /** @type {!Array<number>} */ (reader.isDelimited() ? reader.readPackedUint32() : [reader.readUint32()]);
      for (var i = 0; i < values.length; i++) {
        msg.addKeyIds(values[i]);
      }
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.SpecificKeys.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.SpecificKeys} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getKeyIdsList();
  if (f.length > 0) {
    writer.writePackedUint32(
      1,
      f
    );
  }
};


/**
 * repeated uint32 key_ids = 1;
 * @return {!Array<number>}
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.prototype.getKeyIdsList = function() {
  return /** @type {!Array<number>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * @param {!Array<number>} value
 * @return {!proto.org.dash.platform.dapi.v0.SpecificKeys} returns this
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.prototype.setKeyIdsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {number} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.SpecificKeys} returns this
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.prototype.addKeyIds = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.SpecificKeys} returns this
 */
proto.org.dash.platform.dapi.v0.SpecificKeys.prototype.clearKeyIdsList = function() {
  return this.setKeyIdsList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.SearchKey.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.SearchKey.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.SearchKey} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.SearchKey.toObject = function(includeInstance, msg) {
  var f, obj = {
    purposeMapMap: (f = msg.getPurposeMapMap()) ? f.toObject(includeInstance, proto.org.dash.platform.dapi.v0.SecurityLevelMap.toObject) : []
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.SearchKey}
 */
proto.org.dash.platform.dapi.v0.SearchKey.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.SearchKey;
  return proto.org.dash.platform.dapi.v0.SearchKey.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.SearchKey} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.SearchKey}
 */
proto.org.dash.platform.dapi.v0.SearchKey.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = msg.getPurposeMapMap();
      reader.readMessage(value, function(message, reader) {
        jspb.Map.deserializeBinary(message, reader, jspb.BinaryReader.prototype.readUint32, jspb.BinaryReader.prototype.readMessage, proto.org.dash.platform.dapi.v0.SecurityLevelMap.deserializeBinaryFromReader, 0, new proto.org.dash.platform.dapi.v0.SecurityLevelMap());
         });
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.SearchKey.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.SearchKey.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.SearchKey} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.SearchKey.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPurposeMapMap(true);
  if (f && f.getLength() > 0) {
    f.serializeBinary(1, writer, jspb.BinaryWriter.prototype.writeUint32, jspb.BinaryWriter.prototype.writeMessage, proto.org.dash.platform.dapi.v0.SecurityLevelMap.serializeBinaryToWriter);
  }
};


/**
 * map<uint32, SecurityLevelMap> purpose_map = 1;
 * @param {boolean=} opt_noLazyCreate Do not create the map if
 * empty, instead returning `undefined`
 * @return {!jspb.Map<number,!proto.org.dash.platform.dapi.v0.SecurityLevelMap>}
 */
proto.org.dash.platform.dapi.v0.SearchKey.prototype.getPurposeMapMap = function(opt_noLazyCreate) {
  return /** @type {!jspb.Map<number,!proto.org.dash.platform.dapi.v0.SecurityLevelMap>} */ (
      jspb.Message.getMapField(this, 1, opt_noLazyCreate,
      proto.org.dash.platform.dapi.v0.SecurityLevelMap));
};


/**
 * Clears values from the map. The map will be non-null.
 * @return {!proto.org.dash.platform.dapi.v0.SearchKey} returns this
 */
proto.org.dash.platform.dapi.v0.SearchKey.prototype.clearPurposeMapMap = function() {
  this.getPurposeMapMap().clear();
  return this;};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.SecurityLevelMap.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.SecurityLevelMap} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.toObject = function(includeInstance, msg) {
  var f, obj = {
    securityLevelMapMap: (f = msg.getSecurityLevelMapMap()) ? f.toObject(includeInstance, undefined) : []
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.SecurityLevelMap}
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.SecurityLevelMap;
  return proto.org.dash.platform.dapi.v0.SecurityLevelMap.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.SecurityLevelMap} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.SecurityLevelMap}
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = msg.getSecurityLevelMapMap();
      reader.readMessage(value, function(message, reader) {
        jspb.Map.deserializeBinary(message, reader, jspb.BinaryReader.prototype.readUint32, jspb.BinaryReader.prototype.readEnum, null, 0, 0);
         });
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.SecurityLevelMap.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.SecurityLevelMap} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getSecurityLevelMapMap(true);
  if (f && f.getLength() > 0) {
    f.serializeBinary(1, writer, jspb.BinaryWriter.prototype.writeUint32, jspb.BinaryWriter.prototype.writeEnum);
  }
};


/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType = {
  CURRENT_KEY_OF_KIND_REQUEST: 0,
  ALL_KEYS_OF_KIND_REQUEST: 1
};

/**
 * map<uint32, KeyKindRequestType> security_level_map = 1;
 * @param {boolean=} opt_noLazyCreate Do not create the map if
 * empty, instead returning `undefined`
 * @return {!jspb.Map<number,!proto.org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType>}
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.prototype.getSecurityLevelMapMap = function(opt_noLazyCreate) {
  return /** @type {!jspb.Map<number,!proto.org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType>} */ (
      jspb.Message.getMapField(this, 1, opt_noLazyCreate,
      null));
};


/**
 * Clears values from the map. The map will be non-null.
 * @return {!proto.org.dash.platform.dapi.v0.SecurityLevelMap} returns this
 */
proto.org.dash.platform.dapi.v0.SecurityLevelMap.prototype.clearSecurityLevelMapMap = function() {
  this.getSecurityLevelMapMap().clear();
  return this;};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    identityId: msg.getIdentityId_asB64(),
    requestType: (f = msg.getRequestType()) && proto.org.dash.platform.dapi.v0.KeyRequestType.toObject(includeInstance, f),
    limit: (f = msg.getLimit()) && google_protobuf_wrappers_pb.UInt32Value.toObject(includeInstance, f),
    offset: (f = msg.getOffset()) && google_protobuf_wrappers_pb.UInt32Value.toObject(includeInstance, f),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 5, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setIdentityId(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.KeyRequestType;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.KeyRequestType.deserializeBinaryFromReader);
      msg.setRequestType(value);
      break;
    case 3:
      var value = new google_protobuf_wrappers_pb.UInt32Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt32Value.deserializeBinaryFromReader);
      msg.setLimit(value);
      break;
    case 4:
      var value = new google_protobuf_wrappers_pb.UInt32Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt32Value.deserializeBinaryFromReader);
      msg.setOffset(value);
      break;
    case 5:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentityId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getRequestType();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.KeyRequestType.serializeBinaryToWriter
    );
  }
  f = message.getLimit();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      google_protobuf_wrappers_pb.UInt32Value.serializeBinaryToWriter
    );
  }
  f = message.getOffset();
  if (f != null) {
    writer.writeMessage(
      4,
      f,
      google_protobuf_wrappers_pb.UInt32Value.serializeBinaryToWriter
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      5,
      f
    );
  }
};


/**
 * optional bytes identity_id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.getIdentityId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity_id = 1;
 * This is a type-conversion wrapper around `getIdentityId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.getIdentityId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getIdentityId()));
};


/**
 * optional bytes identity_id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdentityId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.getIdentityId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentityId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.setIdentityId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional KeyRequestType request_type = 2;
 * @return {?proto.org.dash.platform.dapi.v0.KeyRequestType}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.getRequestType = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.KeyRequestType} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.KeyRequestType, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.KeyRequestType|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.setRequestType = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.clearRequestType = function() {
  return this.setRequestType(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.hasRequestType = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional google.protobuf.UInt32Value limit = 3;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.getLimit = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 3));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.setLimit = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.clearLimit = function() {
  return this.setLimit(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.hasLimit = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional google.protobuf.UInt32Value offset = 4;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.getOffset = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 4));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.setOffset = function(value) {
  return jspb.Message.setWrapperField(this, 4, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.clearOffset = function() {
  return this.setOffset(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.hasOffset = function() {
  return jspb.Message.getField(this, 4) != null;
};


/**
 * optional bool prove = 5;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 5, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 5, value);
};


/**
 * optional GetIdentityKeysRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.GetIdentityKeysRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  KEYS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    keys: (f = msg.getKeys()) && proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.deserializeBinaryFromReader);
      msg.setKeys(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getKeys();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.toObject = function(includeInstance, msg) {
  var f, obj = {
    keysBytesList: msg.getKeysBytesList_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys;
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addKeysBytes(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getKeysBytesList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      1,
      f
    );
  }
};


/**
 * repeated bytes keys_bytes = 1;
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.getKeysBytesList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes keys_bytes = 1;
 * This is a type-conversion wrapper around `getKeysBytesList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.getKeysBytesList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getKeysBytesList()));
};


/**
 * repeated bytes keys_bytes = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getKeysBytesList()`
 * @return {!Array<!Uint8Array>}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.getKeysBytesList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getKeysBytesList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.setKeysBytesList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.addKeysBytes = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.prototype.clearKeysBytesList = function() {
  return this.setKeysBytesList([]);
};


/**
 * optional Keys keys = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.getKeys = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.setKeys = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.clearKeys = function() {
  return this.setKeys(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.hasKeys = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetIdentityKeysResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.GetIdentityKeysResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProofsRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProofsRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.repeatedFields_ = [1,2,3];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    identitiesList: jspb.Message.toObjectList(msg.getIdentitiesList(),
    proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.toObject, includeInstance),
    contractsList: jspb.Message.toObjectList(msg.getContractsList(),
    proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.toObject, includeInstance),
    documentsList: jspb.Message.toObjectList(msg.getDocumentsList(),
    proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.deserializeBinaryFromReader);
      msg.addIdentities(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.deserializeBinaryFromReader);
      msg.addContracts(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.deserializeBinaryFromReader);
      msg.addDocuments(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentitiesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.serializeBinaryToWriter
    );
  }
  f = message.getContractsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.serializeBinaryToWriter
    );
  }
  f = message.getDocumentsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    contractId: msg.getContractId_asB64(),
    documentType: jspb.Message.getFieldWithDefault(msg, 2, ""),
    documentTypeKeepsHistory: jspb.Message.getBooleanFieldWithDefault(msg, 3, false),
    documentId: msg.getDocumentId_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setContractId(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setDocumentType(value);
      break;
    case 3:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setDocumentTypeKeepsHistory(value);
      break;
    case 4:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setDocumentId(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getContractId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getDocumentType();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getDocumentTypeKeepsHistory();
  if (f) {
    writer.writeBool(
      3,
      f
    );
  }
  f = message.getDocumentId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      4,
      f
    );
  }
};


/**
 * optional bytes contract_id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getContractId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes contract_id = 1;
 * This is a type-conversion wrapper around `getContractId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getContractId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getContractId()));
};


/**
 * optional bytes contract_id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getContractId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getContractId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getContractId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.setContractId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional string document_type = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getDocumentType = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.setDocumentType = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional bool document_type_keeps_history = 3;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getDocumentTypeKeepsHistory = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 3, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.setDocumentTypeKeepsHistory = function(value) {
  return jspb.Message.setProto3BooleanField(this, 3, value);
};


/**
 * optional bytes document_id = 4;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getDocumentId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * optional bytes document_id = 4;
 * This is a type-conversion wrapper around `getDocumentId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getDocumentId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getDocumentId()));
};


/**
 * optional bytes document_id = 4;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getDocumentId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.getDocumentId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getDocumentId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest.prototype.setDocumentId = function(value) {
  return jspb.Message.setProto3BytesField(this, 4, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    identityId: msg.getIdentityId_asB64(),
    requestType: jspb.Message.getFieldWithDefault(msg, 2, 0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setIdentityId(value);
      break;
    case 2:
      var value = /** @type {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.Type} */ (reader.readEnum());
      msg.setRequestType(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentityId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getRequestType();
  if (f !== 0.0) {
    writer.writeEnum(
      2,
      f
    );
  }
};


/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.Type = {
  FULL_IDENTITY: 0,
  BALANCE: 1,
  KEYS: 2
};

/**
 * optional bytes identity_id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.getIdentityId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity_id = 1;
 * This is a type-conversion wrapper around `getIdentityId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.getIdentityId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getIdentityId()));
};


/**
 * optional bytes identity_id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdentityId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.getIdentityId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentityId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.setIdentityId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional Type request_type = 2;
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.Type}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.getRequestType = function() {
  return /** @type {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.Type} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.Type} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest.prototype.setRequestType = function(value) {
  return jspb.Message.setProto3EnumField(this, 2, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    contractId: msg.getContractId_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setContractId(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getContractId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
};


/**
 * optional bytes contract_id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.prototype.getContractId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes contract_id = 1;
 * This is a type-conversion wrapper around `getContractId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.prototype.getContractId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getContractId()));
};


/**
 * optional bytes contract_id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getContractId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.prototype.getContractId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getContractId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest.prototype.setContractId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * repeated IdentityRequest identities = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest>}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.getIdentitiesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.setIdentitiesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.addIdentities = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.IdentityRequest, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.clearIdentitiesList = function() {
  return this.setIdentitiesList([]);
};


/**
 * repeated ContractRequest contracts = 2;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest>}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.getContractsList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest, 2));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.setContractsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.addContracts = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.ContractRequest, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.clearContractsList = function() {
  return this.setContractsList([]);
};


/**
 * repeated DocumentRequest documents = 3;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest>}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.getDocumentsList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest, 3));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.setDocumentsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 3, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.addDocuments = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 3, opt_value, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.DocumentRequest, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0.prototype.clearDocumentsList = function() {
  return this.setDocumentsList([]);
};


/**
 * optional GetProofsRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProofsRequest.GetProofsRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProofsRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProofsResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProofsResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProofsResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsResponse;
  return proto.org.dash.platform.dapi.v0.GetProofsResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  PROOF: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0;
  return proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional Proof proof = 1;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional ResponseMetadata metadata = 2;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional GetProofsResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProofsResponse.GetProofsResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProofsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractRequest;
  return proto.org.dash.platform.dapi.v0.GetDataContractRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    id: msg.getId_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0;
  return proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setId(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * optional bytes id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.getId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getId()));
};


/**
 * optional bytes id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetDataContractRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractRequest.GetDataContractRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractResponse;
  return proto.org.dash.platform.dapi.v0.GetDataContractResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  DATA_CONTRACT: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    dataContract: msg.getDataContract_asB64(),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0;
  return proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setDataContract(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = /** @type {!(string|Uint8Array)} */ (jspb.Message.getField(message, 1));
  if (f != null) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes data_contract = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.getDataContract = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes data_contract = 1;
 * This is a type-conversion wrapper around `getDataContract()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.getDataContract_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getDataContract()));
};


/**
 * optional bytes data_contract = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getDataContract()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.getDataContract_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getDataContract()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.setDataContract = function(value) {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.clearDataContract = function() {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.hasDataContract = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetDataContractResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractResponse.GetDataContractResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractsRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractsRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractsRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractsRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractsRequest;
  return proto.org.dash.platform.dapi.v0.GetDataContractsRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractsRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    idsList: msg.getIdsList_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0;
  return proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addIds(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdsList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * repeated bytes ids = 1;
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.getIdsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes ids = 1;
 * This is a type-conversion wrapper around `getIdsList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.getIdsList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getIdsList()));
};


/**
 * repeated bytes ids = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdsList()`
 * @return {!Array<!Uint8Array>}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.getIdsList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getIdsList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.setIdsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.addIds = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.clearIdsList = function() {
  return this.setIdsList([]);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetDataContractsRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractsRequest.GetDataContractsRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractsRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractsResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractsResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse;
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.toObject = function(includeInstance, msg) {
  var f, obj = {
    identifier: msg.getIdentifier_asB64(),
    dataContract: (f = msg.getDataContract()) && google_protobuf_wrappers_pb.BytesValue.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry;
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setIdentifier(value);
      break;
    case 2:
      var value = new google_protobuf_wrappers_pb.BytesValue;
      reader.readMessage(value,google_protobuf_wrappers_pb.BytesValue.deserializeBinaryFromReader);
      msg.setDataContract(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentifier_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getDataContract();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      google_protobuf_wrappers_pb.BytesValue.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes identifier = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getIdentifier = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identifier = 1;
 * This is a type-conversion wrapper around `getIdentifier()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getIdentifier_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getIdentifier()));
};


/**
 * optional bytes identifier = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdentifier()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getIdentifier_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentifier()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.setIdentifier = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional google.protobuf.BytesValue data_contract = 2;
 * @return {?proto.google.protobuf.BytesValue}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getDataContract = function() {
  return /** @type{?proto.google.protobuf.BytesValue} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.BytesValue, 2));
};


/**
 * @param {?proto.google.protobuf.BytesValue|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.setDataContract = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.clearDataContract = function() {
  return this.setDataContract(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.hasDataContract = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.toObject = function(includeInstance, msg) {
  var f, obj = {
    dataContractEntriesList: jspb.Message.toObjectList(msg.getDataContractEntriesList(),
    proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts;
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.deserializeBinaryFromReader);
      msg.addDataContractEntries(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDataContractEntriesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.serializeBinaryToWriter
    );
  }
};


/**
 * repeated DataContractEntry data_contract_entries = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry>}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.prototype.getDataContractEntriesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.prototype.setDataContractEntriesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.prototype.addDataContractEntries = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.prototype.clearDataContractEntriesList = function() {
  return this.setDataContractEntriesList([]);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  DATA_CONTRACTS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    dataContracts: (f = msg.getDataContracts()) && proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0;
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.deserializeBinaryFromReader);
      msg.setDataContracts(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDataContracts();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional DataContracts data_contracts = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.getDataContracts = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.setDataContracts = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.clearDataContracts = function() {
  return this.setDataContracts(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.hasDataContracts = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetDataContractsResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.GetDataContractsResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    id: msg.getId_asB64(),
    limit: (f = msg.getLimit()) && google_protobuf_wrappers_pb.UInt32Value.toObject(includeInstance, f),
    offset: (f = msg.getOffset()) && google_protobuf_wrappers_pb.UInt32Value.toObject(includeInstance, f),
    startAtMs: jspb.Message.getFieldWithDefault(msg, 4, 0),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 5, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setId(value);
      break;
    case 2:
      var value = new google_protobuf_wrappers_pb.UInt32Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt32Value.deserializeBinaryFromReader);
      msg.setLimit(value);
      break;
    case 3:
      var value = new google_protobuf_wrappers_pb.UInt32Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt32Value.deserializeBinaryFromReader);
      msg.setOffset(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setStartAtMs(value);
      break;
    case 5:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getLimit();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      google_protobuf_wrappers_pb.UInt32Value.serializeBinaryToWriter
    );
  }
  f = message.getOffset();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      google_protobuf_wrappers_pb.UInt32Value.serializeBinaryToWriter
    );
  }
  f = message.getStartAtMs();
  if (f !== 0) {
    writer.writeUint64(
      4,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      5,
      f
    );
  }
};


/**
 * optional bytes id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.getId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getId()));
};


/**
 * optional bytes id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional google.protobuf.UInt32Value limit = 2;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.getLimit = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 2));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.setLimit = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.clearLimit = function() {
  return this.setLimit(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.hasLimit = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional google.protobuf.UInt32Value offset = 3;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.getOffset = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 3));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.setOffset = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.clearOffset = function() {
  return this.setOffset(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.hasOffset = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional uint64 start_at_ms = 4;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.getStartAtMs = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.setStartAtMs = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional bool prove = 5;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 5, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 5, value);
};


/**
 * optional GetDataContractHistoryRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.GetDataContractHistoryRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  DATA_CONTRACT_HISTORY: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    dataContractHistory: (f = msg.getDataContractHistory()) && proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.deserializeBinaryFromReader);
      msg.setDataContractHistory(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDataContractHistory();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.toObject = function(includeInstance, msg) {
  var f, obj = {
    date: jspb.Message.getFieldWithDefault(msg, 1, 0),
    value: msg.getValue_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setDate(value);
      break;
    case 2:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setValue(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDate();
  if (f !== 0) {
    writer.writeUint64(
      1,
      f
    );
  }
  f = message.getValue_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      2,
      f
    );
  }
};


/**
 * optional uint64 date = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.getDate = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.setDate = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional bytes value = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.getValue = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes value = 2;
 * This is a type-conversion wrapper around `getValue()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.getValue_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getValue()));
};


/**
 * optional bytes value = 2;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getValue()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.getValue_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getValue()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.prototype.setValue = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.toObject = function(includeInstance, msg) {
  var f, obj = {
    dataContractEntriesList: jspb.Message.toObjectList(msg.getDataContractEntriesList(),
    proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.deserializeBinaryFromReader);
      msg.addDataContractEntries(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDataContractEntriesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.serializeBinaryToWriter
    );
  }
};


/**
 * repeated DataContractHistoryEntry data_contract_entries = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry>}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.prototype.getDataContractEntriesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.prototype.setDataContractEntriesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.prototype.addDataContractEntries = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.prototype.clearDataContractEntriesList = function() {
  return this.setDataContractEntriesList([]);
};


/**
 * optional DataContractHistory data_contract_history = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.getDataContractHistory = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.setDataContractHistory = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.clearDataContractHistory = function() {
  return this.setDataContractHistory(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.hasDataContractHistory = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetDataContractHistoryResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.GetDataContractHistoryResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDocumentsRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDocumentsRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDocumentsRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDocumentsRequest;
  return proto.org.dash.platform.dapi.v0.GetDocumentsRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDocumentsRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.oneofGroups_ = [[6,7]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.StartCase = {
  START_NOT_SET: 0,
  START_AFTER: 6,
  START_AT: 7
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.StartCase}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getStartCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.StartCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    dataContractId: msg.getDataContractId_asB64(),
    documentType: jspb.Message.getFieldWithDefault(msg, 2, ""),
    where: msg.getWhere_asB64(),
    orderBy: msg.getOrderBy_asB64(),
    limit: jspb.Message.getFieldWithDefault(msg, 5, 0),
    startAfter: msg.getStartAfter_asB64(),
    startAt: msg.getStartAt_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 8, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0;
  return proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setDataContractId(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setDocumentType(value);
      break;
    case 3:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setWhere(value);
      break;
    case 4:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setOrderBy(value);
      break;
    case 5:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setLimit(value);
      break;
    case 6:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStartAfter(value);
      break;
    case 7:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStartAt(value);
      break;
    case 8:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDataContractId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getDocumentType();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getWhere_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      3,
      f
    );
  }
  f = message.getOrderBy_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      4,
      f
    );
  }
  f = message.getLimit();
  if (f !== 0) {
    writer.writeUint32(
      5,
      f
    );
  }
  f = /** @type {!(string|Uint8Array)} */ (jspb.Message.getField(message, 6));
  if (f != null) {
    writer.writeBytes(
      6,
      f
    );
  }
  f = /** @type {!(string|Uint8Array)} */ (jspb.Message.getField(message, 7));
  if (f != null) {
    writer.writeBytes(
      7,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      8,
      f
    );
  }
};


/**
 * optional bytes data_contract_id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getDataContractId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes data_contract_id = 1;
 * This is a type-conversion wrapper around `getDataContractId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getDataContractId_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getDataContractId()));
};


/**
 * optional bytes data_contract_id = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getDataContractId()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getDataContractId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getDataContractId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setDataContractId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional string document_type = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getDocumentType = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setDocumentType = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional bytes where = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getWhere = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * optional bytes where = 3;
 * This is a type-conversion wrapper around `getWhere()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getWhere_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getWhere()));
};


/**
 * optional bytes where = 3;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getWhere()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getWhere_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getWhere()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setWhere = function(value) {
  return jspb.Message.setProto3BytesField(this, 3, value);
};


/**
 * optional bytes order_by = 4;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getOrderBy = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * optional bytes order_by = 4;
 * This is a type-conversion wrapper around `getOrderBy()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getOrderBy_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getOrderBy()));
};


/**
 * optional bytes order_by = 4;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getOrderBy()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getOrderBy_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getOrderBy()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setOrderBy = function(value) {
  return jspb.Message.setProto3BytesField(this, 4, value);
};


/**
 * optional uint32 limit = 5;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getLimit = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 5, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setLimit = function(value) {
  return jspb.Message.setProto3IntField(this, 5, value);
};


/**
 * optional bytes start_after = 6;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getStartAfter = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 6, ""));
};


/**
 * optional bytes start_after = 6;
 * This is a type-conversion wrapper around `getStartAfter()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getStartAfter_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStartAfter()));
};


/**
 * optional bytes start_after = 6;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStartAfter()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getStartAfter_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStartAfter()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setStartAfter = function(value) {
  return jspb.Message.setOneofField(this, 6, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.clearStartAfter = function() {
  return jspb.Message.setOneofField(this, 6, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.hasStartAfter = function() {
  return jspb.Message.getField(this, 6) != null;
};


/**
 * optional bytes start_at = 7;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getStartAt = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 7, ""));
};


/**
 * optional bytes start_at = 7;
 * This is a type-conversion wrapper around `getStartAt()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getStartAt_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStartAt()));
};


/**
 * optional bytes start_at = 7;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStartAt()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getStartAt_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStartAt()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setStartAt = function(value) {
  return jspb.Message.setOneofField(this, 7, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.clearStartAt = function() {
  return jspb.Message.setOneofField(this, 7, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.hasStartAt = function() {
  return jspb.Message.getField(this, 7) != null;
};


/**
 * optional bool prove = 8;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 8, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 8, value);
};


/**
 * optional GetDocumentsRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDocumentsRequest.GetDocumentsRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDocumentsResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDocumentsResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDocumentsResponse;
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  DOCUMENTS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    documents: (f = msg.getDocuments()) && proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0;
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.deserializeBinaryFromReader);
      msg.setDocuments(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDocuments();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.toObject = function(includeInstance, msg) {
  var f, obj = {
    documentsList: msg.getDocumentsList_asB64()
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents;
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addDocuments(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDocumentsList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      1,
      f
    );
  }
};


/**
 * repeated bytes documents = 1;
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.getDocumentsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes documents = 1;
 * This is a type-conversion wrapper around `getDocumentsList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.getDocumentsList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getDocumentsList()));
};


/**
 * repeated bytes documents = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getDocumentsList()`
 * @return {!Array<!Uint8Array>}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.getDocumentsList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getDocumentsList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.setDocumentsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.addDocuments = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents.prototype.clearDocumentsList = function() {
  return this.setDocumentsList([]);
};


/**
 * optional Documents documents = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.getDocuments = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.Documents|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.setDocuments = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.clearDocuments = function() {
  return this.setDocuments(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.hasDocuments = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetDocumentsResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.GetDocumentsResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    publicKeyHashesList: msg.getPublicKeyHashesList_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addPublicKeyHashes(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPublicKeyHashesList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * repeated bytes public_key_hashes = 1;
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.getPublicKeyHashesList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes public_key_hashes = 1;
 * This is a type-conversion wrapper around `getPublicKeyHashesList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.getPublicKeyHashesList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getPublicKeyHashesList()));
};


/**
 * repeated bytes public_key_hashes = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getPublicKeyHashesList()`
 * @return {!Array<!Uint8Array>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.getPublicKeyHashesList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getPublicKeyHashesList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.setPublicKeyHashesList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.addPublicKeyHashes = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.clearPublicKeyHashesList = function() {
  return this.setPublicKeyHashesList([]);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetIdentitiesByPublicKeyHashesRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.toObject = function(includeInstance, msg) {
  var f, obj = {
    publicKeyHash: msg.getPublicKeyHash_asB64(),
    value: (f = msg.getValue()) && google_protobuf_wrappers_pb.BytesValue.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setPublicKeyHash(value);
      break;
    case 2:
      var value = new google_protobuf_wrappers_pb.BytesValue;
      reader.readMessage(value,google_protobuf_wrappers_pb.BytesValue.deserializeBinaryFromReader);
      msg.setValue(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPublicKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getValue();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      google_protobuf_wrappers_pb.BytesValue.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes public_key_hash = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.getPublicKeyHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes public_key_hash = 1;
 * This is a type-conversion wrapper around `getPublicKeyHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.getPublicKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getPublicKeyHash()));
};


/**
 * optional bytes public_key_hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getPublicKeyHash()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.getPublicKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getPublicKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.setPublicKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional google.protobuf.BytesValue value = 2;
 * @return {?proto.google.protobuf.BytesValue}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.getValue = function() {
  return /** @type{?proto.google.protobuf.BytesValue} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.BytesValue, 2));
};


/**
 * @param {?proto.google.protobuf.BytesValue|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.setValue = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.clearValue = function() {
  return this.setValue(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.prototype.hasValue = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.toObject = function(includeInstance, msg) {
  var f, obj = {
    identityEntriesList: jspb.Message.toObjectList(msg.getIdentityEntriesList(),
    proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.deserializeBinaryFromReader);
      msg.addIdentityEntries(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentityEntriesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.serializeBinaryToWriter
    );
  }
};


/**
 * repeated PublicKeyHashIdentityEntry identity_entries = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.prototype.getIdentityEntriesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.prototype.setIdentityEntriesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.prototype.addIdentityEntries = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.prototype.clearIdentityEntriesList = function() {
  return this.setIdentityEntriesList([]);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITIES: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    identities: (f = msg.getIdentities()) && proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.deserializeBinaryFromReader);
      msg.setIdentities(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentities();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional IdentitiesByPublicKeyHashes identities = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.getIdentities = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.setIdentities = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.clearIdentities = function() {
  return this.setIdentities(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.hasIdentities = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetIdentitiesByPublicKeyHashesResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    publicKeyHash: msg.getPublicKeyHash_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setPublicKeyHash(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPublicKeyHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * optional bytes public_key_hash = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.getPublicKeyHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes public_key_hash = 1;
 * This is a type-conversion wrapper around `getPublicKeyHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.getPublicKeyHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getPublicKeyHash()));
};


/**
 * optional bytes public_key_hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getPublicKeyHash()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.getPublicKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getPublicKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.setPublicKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetIdentityByPublicKeyHashRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITY: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    identity: msg.getIdentity_asB64(),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0;
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setIdentity(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = /** @type {!(string|Uint8Array)} */ (jspb.Message.getField(message, 1));
  if (f != null) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes identity = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.getIdentity = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity = 1;
 * This is a type-conversion wrapper around `getIdentity()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.getIdentity_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getIdentity()));
};


/**
 * optional bytes identity = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdentity()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.getIdentity_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentity()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.setIdentity = function(value) {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.clearIdentity = function() {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.hasIdentity = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetIdentityByPublicKeyHashResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest;
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    stateTransitionHash: msg.getStateTransitionHash_asB64(),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0;
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStateTransitionHash(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getStateTransitionHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * optional bytes state_transition_hash = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.getStateTransitionHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes state_transition_hash = 1;
 * This is a type-conversion wrapper around `getStateTransitionHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.getStateTransitionHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStateTransitionHash()));
};


/**
 * optional bytes state_transition_hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStateTransitionHash()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.getStateTransitionHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateTransitionHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.setStateTransitionHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional WaitForStateTransitionResultRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse;
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  ERROR: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    error: (f = msg.getError()) && proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0;
  return proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.deserializeBinaryFromReader);
      msg.setError(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getError();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};


/**
 * optional StateTransitionBroadcastError error = 1;
 * @return {?proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.getError = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.setError = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.clearError = function() {
  return this.setError(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.hasError = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional WaitForStateTransitionResultResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest;
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    height: jspb.Message.getFieldWithDefault(msg, 1, 0),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 2, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0;
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readInt32());
      msg.setHeight(value);
      break;
    case 2:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getHeight();
  if (f !== 0) {
    writer.writeInt32(
      1,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      2,
      f
    );
  }
};


/**
 * optional int32 height = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.prototype.getHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.prototype.setHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};


/**
 * optional GetConsensusParamsRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.GetConsensusParamsRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse;
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.toObject = function(includeInstance, msg) {
  var f, obj = {
    maxBytes: jspb.Message.getFieldWithDefault(msg, 1, ""),
    maxGas: jspb.Message.getFieldWithDefault(msg, 2, ""),
    timeIotaMs: jspb.Message.getFieldWithDefault(msg, 3, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock;
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setMaxBytes(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setMaxGas(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.setTimeIotaMs(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getMaxBytes();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getMaxGas();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getTimeIotaMs();
  if (f.length > 0) {
    writer.writeString(
      3,
      f
    );
  }
};


/**
 * optional string max_bytes = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.getMaxBytes = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.setMaxBytes = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string max_gas = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.getMaxGas = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.setMaxGas = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string time_iota_ms = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.getTimeIotaMs = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.prototype.setTimeIotaMs = function(value) {
  return jspb.Message.setProto3StringField(this, 3, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.toObject = function(includeInstance, msg) {
  var f, obj = {
    maxAgeNumBlocks: jspb.Message.getFieldWithDefault(msg, 1, ""),
    maxAgeDuration: jspb.Message.getFieldWithDefault(msg, 2, ""),
    maxBytes: jspb.Message.getFieldWithDefault(msg, 3, "")
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence;
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {string} */ (reader.readString());
      msg.setMaxAgeNumBlocks(value);
      break;
    case 2:
      var value = /** @type {string} */ (reader.readString());
      msg.setMaxAgeDuration(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.setMaxBytes(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getMaxAgeNumBlocks();
  if (f.length > 0) {
    writer.writeString(
      1,
      f
    );
  }
  f = message.getMaxAgeDuration();
  if (f.length > 0) {
    writer.writeString(
      2,
      f
    );
  }
  f = message.getMaxBytes();
  if (f.length > 0) {
    writer.writeString(
      3,
      f
    );
  }
};


/**
 * optional string max_age_num_blocks = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.getMaxAgeNumBlocks = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.setMaxAgeNumBlocks = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string max_age_duration = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.getMaxAgeDuration = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.setMaxAgeDuration = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string max_bytes = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.getMaxBytes = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.prototype.setMaxBytes = function(value) {
  return jspb.Message.setProto3StringField(this, 3, value);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    block: (f = msg.getBlock()) && proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.toObject(includeInstance, f),
    evidence: (f = msg.getEvidence()) && proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0;
  return proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.deserializeBinaryFromReader);
      msg.setBlock(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.deserializeBinaryFromReader);
      msg.setEvidence(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getBlock();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock.serializeBinaryToWriter
    );
  }
  f = message.getEvidence();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence.serializeBinaryToWriter
    );
  }
};


/**
 * optional ConsensusParamsBlock block = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.getBlock = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsBlock|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.setBlock = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.clearBlock = function() {
  return this.setBlock(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.hasBlock = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional ConsensusParamsEvidence evidence = 2;
 * @return {?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.getEvidence = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.ConsensusParamsEvidence|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.setEvidence = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.clearEvidence = function() {
  return this.setEvidence(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0.prototype.hasEvidence = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional GetConsensusParamsResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.GetConsensusParamsResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 1, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getProve();
  if (f) {
    writer.writeBool(
      1,
      f
    );
  }
};


/**
 * optional bool prove = 1;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 1, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 1, value);
};


/**
 * optional GetProtocolVersionUpgradeStateRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  VERSIONS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    versions: (f = msg.getVersions()) && proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.deserializeBinaryFromReader);
      msg.setVersions(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getVersions();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.toObject = function(includeInstance, msg) {
  var f, obj = {
    versionsList: jspb.Message.toObjectList(msg.getVersionsList(),
    proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.deserializeBinaryFromReader);
      msg.addVersions(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getVersionsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.serializeBinaryToWriter
    );
  }
};


/**
 * repeated VersionEntry versions = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry>}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.prototype.getVersionsList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.prototype.setVersionsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.prototype.addVersions = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.prototype.clearVersionsList = function() {
  return this.setVersionsList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.toObject = function(includeInstance, msg) {
  var f, obj = {
    versionNumber: jspb.Message.getFieldWithDefault(msg, 1, 0),
    voteCount: jspb.Message.getFieldWithDefault(msg, 2, 0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setVersionNumber(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setVoteCount(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getVersionNumber();
  if (f !== 0) {
    writer.writeUint32(
      1,
      f
    );
  }
  f = message.getVoteCount();
  if (f !== 0) {
    writer.writeUint32(
      2,
      f
    );
  }
};


/**
 * optional uint32 version_number = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.prototype.getVersionNumber = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.prototype.setVersionNumber = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional uint32 vote_count = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.prototype.getVoteCount = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.prototype.setVoteCount = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional Versions versions = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.getVersions = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.setVersions = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.clearVersions = function() {
  return this.setVersions(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.hasVersions = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetProtocolVersionUpgradeStateResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeStateResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    startProTxHash: msg.getStartProTxHash_asB64(),
    count: jspb.Message.getFieldWithDefault(msg, 2, 0),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 3, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setStartProTxHash(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setCount(value);
      break;
    case 3:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getStartProTxHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getCount();
  if (f !== 0) {
    writer.writeUint32(
      2,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      3,
      f
    );
  }
};


/**
 * optional bytes start_pro_tx_hash = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.getStartProTxHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes start_pro_tx_hash = 1;
 * This is a type-conversion wrapper around `getStartProTxHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.getStartProTxHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getStartProTxHash()));
};


/**
 * optional bytes start_pro_tx_hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getStartProTxHash()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.getStartProTxHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStartProTxHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.setStartProTxHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional uint32 count = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.getCount = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.setCount = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional bool prove = 3;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 3, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 3, value);
};


/**
 * optional GetProtocolVersionUpgradeVoteStatusRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  VERSIONS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    versions: (f = msg.getVersions()) && proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.deserializeBinaryFromReader);
      msg.setVersions(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getVersions();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.toObject = function(includeInstance, msg) {
  var f, obj = {
    versionSignalsList: jspb.Message.toObjectList(msg.getVersionSignalsList(),
    proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.deserializeBinaryFromReader);
      msg.addVersionSignals(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getVersionSignalsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.serializeBinaryToWriter
    );
  }
};


/**
 * repeated VersionSignal version_signals = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal>}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.prototype.getVersionSignalsList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.prototype.setVersionSignalsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.prototype.addVersionSignals = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.prototype.clearVersionSignalsList = function() {
  return this.setVersionSignalsList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.toObject = function(includeInstance, msg) {
  var f, obj = {
    proTxHash: msg.getProTxHash_asB64(),
    version: jspb.Message.getFieldWithDefault(msg, 2, 0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal;
  return proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setProTxHash(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setVersion(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getProTxHash_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getVersion();
  if (f !== 0) {
    writer.writeUint32(
      2,
      f
    );
  }
};


/**
 * optional bytes pro_tx_hash = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.getProTxHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes pro_tx_hash = 1;
 * This is a type-conversion wrapper around `getProTxHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.getProTxHash_asB64 = function() {
  return /** @type {string} */ (jspb.Message.bytesAsB64(
      this.getProTxHash()));
};


/**
 * optional bytes pro_tx_hash = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getProTxHash()`
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.getProTxHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getProTxHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.setProTxHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional uint32 version = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.getVersion = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.prototype.setVersion = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional VersionSignals versions = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.getVersions = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.setVersions = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.clearVersions = function() {
  return this.setVersions(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.hasVersions = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetProtocolVersionUpgradeVoteStatusResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProtocolVersionUpgradeVoteStatusResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest;
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.serializeBinaryToWriter
    );
  }
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    startEpoch: (f = msg.getStartEpoch()) && google_protobuf_wrappers_pb.UInt32Value.toObject(includeInstance, f),
    count: jspb.Message.getFieldWithDefault(msg, 2, 0),
    ascending: jspb.Message.getBooleanFieldWithDefault(msg, 3, false),
    prove: jspb.Message.getBooleanFieldWithDefault(msg, 4, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0;
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new google_protobuf_wrappers_pb.UInt32Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt32Value.deserializeBinaryFromReader);
      msg.setStartEpoch(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setCount(value);
      break;
    case 3:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setAscending(value);
      break;
    case 4:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setProve(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getStartEpoch();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      google_protobuf_wrappers_pb.UInt32Value.serializeBinaryToWriter
    );
  }
  f = message.getCount();
  if (f !== 0) {
    writer.writeUint32(
      2,
      f
    );
  }
  f = message.getAscending();
  if (f) {
    writer.writeBool(
      3,
      f
    );
  }
  f = message.getProve();
  if (f) {
    writer.writeBool(
      4,
      f
    );
  }
};


/**
 * optional google.protobuf.UInt32Value start_epoch = 1;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.getStartEpoch = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 1));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.setStartEpoch = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.clearStartEpoch = function() {
  return this.setStartEpoch(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.hasStartEpoch = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional uint32 count = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.getCount = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.setCount = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional bool ascending = 3;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.getAscending = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 3, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.setAscending = function(value) {
  return jspb.Message.setProto3BooleanField(this, 3, value);
};


/**
 * optional bool prove = 4;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 4, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 4, value);
};


/**
 * optional GetEpochsInfoRequestV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.GetEpochsInfoRequestV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoRequest.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.oneofGroups_ = [[1]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.VersionCase = {
  VERSION_NOT_SET: 0,
  V0: 1
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.VersionCase}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.prototype.getVersionCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.VersionCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    v0: (f = msg.getV0()) && proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse;
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.deserializeBinaryFromReader);
      msg.setV0(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getV0();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.serializeBinaryToWriter
    );
  }
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.ResultCase = {
  RESULT_NOT_SET: 0,
  EPOCHS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.oneofGroups_[0]));
};



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.toObject = function(includeInstance, msg) {
  var f, obj = {
    epochs: (f = msg.getEpochs()) && proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.toObject(includeInstance, f),
    proof: (f = msg.getProof()) && proto.org.dash.platform.dapi.v0.Proof.toObject(includeInstance, f),
    metadata: (f = msg.getMetadata()) && proto.org.dash.platform.dapi.v0.ResponseMetadata.toObject(includeInstance, f)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0;
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.deserializeBinaryFromReader);
      msg.setEpochs(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.Proof;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.Proof.deserializeBinaryFromReader);
      msg.setProof(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.ResponseMetadata;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ResponseMetadata.deserializeBinaryFromReader);
      msg.setMetadata(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getEpochs();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.serializeBinaryToWriter
    );
  }
  f = message.getProof();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.Proof.serializeBinaryToWriter
    );
  }
  f = message.getMetadata();
  if (f != null) {
    writer.writeMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.ResponseMetadata.serializeBinaryToWriter
    );
  }
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.repeatedFields_ = [1];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.toObject = function(includeInstance, msg) {
  var f, obj = {
    epochInfosList: jspb.Message.toObjectList(msg.getEpochInfosList(),
    proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos;
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.deserializeBinaryFromReader);
      msg.addEpochInfos(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getEpochInfosList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.serializeBinaryToWriter
    );
  }
};


/**
 * repeated EpochInfo epoch_infos = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo>}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.prototype.getEpochInfosList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos} returns this
*/
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.prototype.setEpochInfosList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.prototype.addEpochInfos = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.prototype.clearEpochInfosList = function() {
  return this.setEpochInfosList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.toObject = function(includeInstance, msg) {
  var f, obj = {
    number: jspb.Message.getFieldWithDefault(msg, 1, 0),
    firstBlockHeight: jspb.Message.getFieldWithDefault(msg, 2, 0),
    firstCoreBlockHeight: jspb.Message.getFieldWithDefault(msg, 3, 0),
    startTime: jspb.Message.getFieldWithDefault(msg, 4, 0),
    feeMultiplier: jspb.Message.getFloatingPointFieldWithDefault(msg, 5, 0.0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo;
  return proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setNumber(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setFirstBlockHeight(value);
      break;
    case 3:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setFirstCoreBlockHeight(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setStartTime(value);
      break;
    case 5:
      var value = /** @type {number} */ (reader.readDouble());
      msg.setFeeMultiplier(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getNumber();
  if (f !== 0) {
    writer.writeUint32(
      1,
      f
    );
  }
  f = message.getFirstBlockHeight();
  if (f !== 0) {
    writer.writeUint64(
      2,
      f
    );
  }
  f = message.getFirstCoreBlockHeight();
  if (f !== 0) {
    writer.writeUint32(
      3,
      f
    );
  }
  f = message.getStartTime();
  if (f !== 0) {
    writer.writeUint64(
      4,
      f
    );
  }
  f = message.getFeeMultiplier();
  if (f !== 0.0) {
    writer.writeDouble(
      5,
      f
    );
  }
};


/**
 * optional uint32 number = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.getNumber = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.setNumber = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional uint64 first_block_height = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.getFirstBlockHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.setFirstBlockHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional uint32 first_core_block_height = 3;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.getFirstCoreBlockHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 3, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.setFirstCoreBlockHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 3, value);
};


/**
 * optional uint64 start_time = 4;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.getStartTime = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.setStartTime = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional double fee_multiplier = 5;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.getFeeMultiplier = function() {
  return /** @type {number} */ (jspb.Message.getFloatingPointFieldWithDefault(this, 5, 0.0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.prototype.setFeeMultiplier = function(value) {
  return jspb.Message.setProto3FloatField(this, 5, value);
};


/**
 * optional EpochInfos epochs = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.getEpochs = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.setEpochs = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.clearEpochs = function() {
  return this.setEpochs(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.hasEpochs = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} returns this
*/
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional GetEpochsInfoResponseV0 v0 = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.prototype.getV0 = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.GetEpochsInfoResponseV0|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.prototype.setV0 = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.prototype.clearV0 = function() {
  return this.setV0(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetEpochsInfoResponse.prototype.hasV0 = function() {
  return jspb.Message.getField(this, 1) != null;
};


goog.object.extend(exports, proto.org.dash.platform.dapi.v0);
