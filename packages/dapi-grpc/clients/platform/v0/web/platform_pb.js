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
goog.exportSymbol('proto.org.dash.platform.dapi.v0.ConsensusParamsBlock', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDataContractsResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsRequest.StartCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetDocumentsResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityEntry', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.IdentityValue', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetIdentityResponse.ResultCase', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.Type', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.GetProofsResponse', null, { proto });
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
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse', null, { proto });
goog.exportSymbol('proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.ResultCase', null, { proto });
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
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
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
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.repeatedFields_, null);
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
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision';
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
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
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
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys';
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest';
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap';
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse';
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey';
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry';
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries';
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
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetProofsRequest.repeatedFields_, null);
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest';
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest';
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest';
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
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
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
proto.org.dash.platform.dapi.v0.GetDataContractRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
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
proto.org.dash.platform.dapi.v0.GetDataContractsRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetDataContractsRequest.repeatedFields_, null);
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
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue';
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry';
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.displayName = 'proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory';
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
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.displayName = 'proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents';
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
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.repeatedFields_, null);
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
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.repeatedFields_, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities';
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest';
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.oneofGroups_);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse';
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
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
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
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.ConsensusParamsBlock, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.displayName = 'proto.org.dash.platform.dapi.v0.ConsensusParamsBlock';
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
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.displayName = 'proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence';
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
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
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
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.displayName = 'proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse';
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
    timeMs: jspb.Message.getFieldWithDefault(msg, 3, 0),
    protocolVersion: jspb.Message.getFieldWithDefault(msg, 4, 0),
    chainId: jspb.Message.getFieldWithDefault(msg, 5, "")
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
      var value = /** @type {number} */ (reader.readUint64());
      msg.setTimeMs(value);
      break;
    case 4:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setProtocolVersion(value);
      break;
    case 5:
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
  f = message.getTimeMs();
  if (f !== 0) {
    writer.writeUint64(
      3,
      f
    );
  }
  f = message.getProtocolVersion();
  if (f !== 0) {
    writer.writeUint32(
      4,
      f
    );
  }
  f = message.getChainId();
  if (f.length > 0) {
    writer.writeString(
      5,
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
 * optional uint64 time_ms = 3;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getTimeMs = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 3, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setTimeMs = function(value) {
  return jspb.Message.setProto3IntField(this, 3, value);
};


/**
 * optional uint32 protocol_version = 4;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getProtocolVersion = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setProtocolVersion = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional string chain_id = 5;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.getChainId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 5, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ResponseMetadata} returns this
 */
proto.org.dash.platform.dapi.v0.ResponseMetadata.prototype.setChainId = function(value) {
  return jspb.Message.setProto3StringField(this, 5, value);
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
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.getId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITY: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_[0]));
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
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getIdentity = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity = 1;
 * This is a type-conversion wrapper around `getIdentity()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getIdentity_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getIdentity_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentity()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.setIdentity = function(value) {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.clearIdentity = function() {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.hasIdentity = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.getIdsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes ids = 1;
 * This is a type-conversion wrapper around `getIdsList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.getIdsList_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.getIdsList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getIdsList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.setIdsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.addIds = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.clearIdsList = function() {
  return this.setIdsList([]);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITIES: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_[0]));
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
 * optional Identities identities = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.getIdentities = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.Identities|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.setIdentities = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.clearIdentities = function() {
  return this.setIdentities(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.hasIdentities = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  BALANCE: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_[0]));
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
    balance: (f = msg.getBalance()) && google_protobuf_wrappers_pb.UInt64Value.toObject(includeInstance, f),
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
      var value = new google_protobuf_wrappers_pb.UInt64Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt64Value.deserializeBinaryFromReader);
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
  f = message.getBalance();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      google_protobuf_wrappers_pb.UInt64Value.serializeBinaryToWriter
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
 * optional google.protobuf.UInt64Value balance = 1;
 * @return {?proto.google.protobuf.UInt64Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.getBalance = function() {
  return /** @type{?proto.google.protobuf.UInt64Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt64Value, 1));
};


/**
 * @param {?proto.google.protobuf.UInt64Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.setBalance = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.clearBalance = function() {
  return this.setBalance(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.hasBalance = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  BALANCE_AND_REVISION: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_[0]));
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
    balanceAndRevision: (f = msg.getBalanceAndRevision()) && proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.toObject(includeInstance, f),
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
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.deserializeBinaryFromReader);
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
  f = message.getBalanceAndRevision();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.serializeBinaryToWriter
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
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.toObject = function(includeInstance, msg) {
  var f, obj = {
    balance: (f = msg.getBalance()) && google_protobuf_wrappers_pb.UInt64Value.toObject(includeInstance, f),
    revision: (f = msg.getRevision()) && google_protobuf_wrappers_pb.UInt64Value.toObject(includeInstance, f)
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision;
  return proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new google_protobuf_wrappers_pb.UInt64Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt64Value.deserializeBinaryFromReader);
      msg.setBalance(value);
      break;
    case 2:
      var value = new google_protobuf_wrappers_pb.UInt64Value;
      reader.readMessage(value,google_protobuf_wrappers_pb.UInt64Value.deserializeBinaryFromReader);
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
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getBalance();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      google_protobuf_wrappers_pb.UInt64Value.serializeBinaryToWriter
    );
  }
  f = message.getRevision();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      google_protobuf_wrappers_pb.UInt64Value.serializeBinaryToWriter
    );
  }
};


/**
 * optional google.protobuf.UInt64Value balance = 1;
 * @return {?proto.google.protobuf.UInt64Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.getBalance = function() {
  return /** @type{?proto.google.protobuf.UInt64Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt64Value, 1));
};


/**
 * @param {?proto.google.protobuf.UInt64Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.setBalance = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.clearBalance = function() {
  return this.setBalance(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.hasBalance = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional google.protobuf.UInt64Value revision = 2;
 * @return {?proto.google.protobuf.UInt64Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.getRevision = function() {
  return /** @type{?proto.google.protobuf.UInt64Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt64Value, 2));
};


/**
 * @param {?proto.google.protobuf.UInt64Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.setRevision = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.clearRevision = function() {
  return this.setRevision(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision.prototype.hasRevision = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional BalanceAndRevision balance_and_revision = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.getBalanceAndRevision = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.BalanceAndRevision|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.setBalanceAndRevision = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.clearBalanceAndRevision = function() {
  return this.setBalanceAndRevision(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.hasBalanceAndRevision = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
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
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getIdentityId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity_id = 1;
 * This is a type-conversion wrapper around `getIdentityId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getIdentityId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getIdentityId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentityId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.setIdentityId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional KeyRequestType request_type = 2;
 * @return {?proto.org.dash.platform.dapi.v0.KeyRequestType}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getRequestType = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.KeyRequestType} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.KeyRequestType, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.KeyRequestType|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.setRequestType = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.clearRequestType = function() {
  return this.setRequestType(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.hasRequestType = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional google.protobuf.UInt32Value limit = 3;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getLimit = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 3));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.setLimit = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.clearLimit = function() {
  return this.setLimit(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.hasLimit = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional google.protobuf.UInt32Value offset = 4;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getOffset = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 4));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.setOffset = function(value) {
  return jspb.Message.setWrapperField(this, 4, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.clearOffset = function() {
  return this.setOffset(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.hasOffset = function() {
  return jspb.Message.getField(this, 4) != null;
};


/**
 * optional bool prove = 5;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 5, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 5, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  KEYS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_[0]));
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
    keys: (f = msg.getKeys()) && proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.toObject(includeInstance, f),
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
      var value = new proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.deserializeBinaryFromReader);
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
  f = message.getKeys();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.serializeBinaryToWriter
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
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys;
  return proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.getKeysBytesList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes keys_bytes = 1;
 * This is a type-conversion wrapper around `getKeysBytesList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.getKeysBytesList_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.getKeysBytesList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getKeysBytesList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.setKeysBytesList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.addKeysBytes = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.prototype.clearKeysBytesList = function() {
  return this.setKeysBytesList([]);
};


/**
 * optional Keys keys = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.getKeys = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.setKeys = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.clearKeys = function() {
  return this.setKeys(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.hasKeys = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityKeysResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.toObject = function(includeInstance, msg) {
  var f, obj = {
    identityIdsList: msg.getIdentityIdsList_asB64(),
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addIdentityIds(value);
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentityIdsList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getSecurityLevelMapMap(true);
  if (f && f.getLength() > 0) {
    f.serializeBinary(1, writer, jspb.BinaryWriter.prototype.writeUint32, jspb.BinaryWriter.prototype.writeEnum);
  }
};


/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType = {
  CURRENT_KEY_OF_KIND_REQUEST: 0
};

/**
 * map<uint32, KeyKindRequestType> security_level_map = 1;
 * @param {boolean=} opt_noLazyCreate Do not create the map if
 * empty, instead returning `undefined`
 * @return {!jspb.Map<number,!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.prototype.getSecurityLevelMapMap = function(opt_noLazyCreate) {
  return /** @type {!jspb.Map<number,!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType>} */ (
      jspb.Message.getMapField(this, 1, opt_noLazyCreate,
      null));
};


/**
 * Clears values from the map. The map will be non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.prototype.clearSecurityLevelMapMap = function() {
  this.getSecurityLevelMapMap().clear();
  return this;};


/**
 * repeated bytes identity_ids = 1;
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.getIdentityIdsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes identity_ids = 1;
 * This is a type-conversion wrapper around `getIdentityIdsList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.getIdentityIdsList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getIdentityIdsList()));
};


/**
 * repeated bytes identity_ids = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdentityIdsList()`
 * @return {!Array<!Uint8Array>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.getIdentityIdsList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getIdentityIdsList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.setIdentityIdsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.addIdentityIds = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.clearIdentityIdsList = function() {
  return this.setIdentityIdsList([]);
};


/**
 * optional KeyRequestType request_type = 2;
 * @return {?proto.org.dash.platform.dapi.v0.KeyRequestType}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.getRequestType = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.KeyRequestType} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.KeyRequestType, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.KeyRequestType|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.setRequestType = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.clearRequestType = function() {
  return this.setRequestType(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.hasRequestType = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional google.protobuf.UInt32Value limit = 3;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.getLimit = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 3));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.setLimit = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.clearLimit = function() {
  return this.setLimit(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.hasLimit = function() {
  return jspb.Message.getField(this, 3) != null;
};


/**
 * optional google.protobuf.UInt32Value offset = 4;
 * @return {?proto.google.protobuf.UInt32Value}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.getOffset = function() {
  return /** @type{?proto.google.protobuf.UInt32Value} */ (
    jspb.Message.getWrapperField(this, google_protobuf_wrappers_pb.UInt32Value, 4));
};


/**
 * @param {?proto.google.protobuf.UInt32Value|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.setOffset = function(value) {
  return jspb.Message.setWrapperField(this, 4, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.clearOffset = function() {
  return this.setOffset(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.hasOffset = function() {
  return jspb.Message.getField(this, 4) != null;
};


/**
 * optional bool prove = 5;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 5, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 5, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  PUBLIC_KEYS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.oneofGroups_[0]));
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.toObject = function(includeInstance, msg) {
  var f, obj = {
    publicKeys: (f = msg.getPublicKeys()) && proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.toObject(includeInstance, f),
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.deserializeBinaryFromReader);
      msg.setPublicKeys(value);
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPublicKeys();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.serializeBinaryToWriter
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.prototype.getValue = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes value = 1;
 * This is a type-conversion wrapper around `getValue()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.prototype.getValue_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.prototype.getValue_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getValue()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.prototype.setValue = function(value) {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.toObject = function(includeInstance, msg) {
  var f, obj = {
    key: msg.getKey_asB64(),
    value: (f = msg.getValue()) && proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.toObject(includeInstance, f)
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.deserializeBinaryFromReader = function(msg, reader) {
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
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.deserializeBinaryFromReader);
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.serializeBinaryToWriter = function(message, writer) {
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
      proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes key = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.getKey = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes key = 1;
 * This is a type-conversion wrapper around `getKey()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.getKey_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.getKey_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getKey()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.setKey = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional PublicKey value = 2;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.getValue = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.setValue = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.clearValue = function() {
  return this.setValue(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.prototype.hasValue = function() {
  return jspb.Message.getField(this, 2) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.toObject = function(includeInstance, msg) {
  var f, obj = {
    publicKeyEntriesList: jspb.Message.toObjectList(msg.getPublicKeyEntriesList(),
    proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.toObject, includeInstance)
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.deserializeBinaryFromReader);
      msg.addPublicKeyEntries(value);
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
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPublicKeyEntriesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.serializeBinaryToWriter
    );
  }
};


/**
 * repeated PublicKeyEntry public_key_entries = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.prototype.getPublicKeyEntriesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.prototype.setPublicKeyEntriesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.prototype.addPublicKeyEntries = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.prototype.clearPublicKeyEntriesList = function() {
  return this.setPublicKeyEntriesList([]);
};


/**
 * optional PublicKeyEntries public_keys = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.getPublicKeys = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.setPublicKeys = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.clearPublicKeys = function() {
  return this.setPublicKeys(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.hasPublicKeys = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.repeatedFields_ = [1,2,3];



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
    identitiesList: jspb.Message.toObjectList(msg.getIdentitiesList(),
    proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.toObject, includeInstance),
    contractsList: jspb.Message.toObjectList(msg.getContractsList(),
    proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.toObject, includeInstance),
    documentsList: jspb.Message.toObjectList(msg.getDocumentsList(),
    proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.toObject, includeInstance)
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
      var value = new proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.deserializeBinaryFromReader);
      msg.addIdentities(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.deserializeBinaryFromReader);
      msg.addContracts(value);
      break;
    case 3:
      var value = new proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.deserializeBinaryFromReader);
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
  f = message.getIdentitiesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.serializeBinaryToWriter
    );
  }
  f = message.getContractsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.serializeBinaryToWriter
    );
  }
  f = message.getDocumentsList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      3,
      f,
      proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.serializeBinaryToWriter
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getContractId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes contract_id = 1;
 * This is a type-conversion wrapper around `getContractId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getContractId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getContractId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getContractId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.setContractId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional string document_type = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getDocumentType = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.setDocumentType = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional bool document_type_keeps_history = 3;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getDocumentTypeKeepsHistory = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 3, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.setDocumentTypeKeepsHistory = function(value) {
  return jspb.Message.setProto3BooleanField(this, 3, value);
};


/**
 * optional bytes document_id = 4;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getDocumentId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * optional bytes document_id = 4;
 * This is a type-conversion wrapper around `getDocumentId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getDocumentId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.getDocumentId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getDocumentId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest.prototype.setDocumentId = function(value) {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.deserializeBinaryFromReader = function(msg, reader) {
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
      var value = /** @type {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.Type} */ (reader.readEnum());
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.Type = {
  FULL_IDENTITY: 0,
  BALANCE: 1,
  KEYS: 2
};

/**
 * optional bytes identity_id = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.getIdentityId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity_id = 1;
 * This is a type-conversion wrapper around `getIdentityId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.getIdentityId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.getIdentityId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentityId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.setIdentityId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional Type request_type = 2;
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.Type}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.getRequestType = function() {
  return /** @type {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.Type} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.Type} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest.prototype.setRequestType = function(value) {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest;
  return proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.prototype.getContractId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes contract_id = 1;
 * This is a type-conversion wrapper around `getContractId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.prototype.getContractId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.prototype.getContractId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getContractId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest.prototype.setContractId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * repeated IdentityRequest identities = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest>}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.getIdentitiesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.setIdentitiesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.addIdentities = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetProofsRequest.IdentityRequest, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.clearIdentitiesList = function() {
  return this.setIdentitiesList([]);
};


/**
 * repeated ContractRequest contracts = 2;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest>}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.getContractsList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest, 2));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.setContractsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.addContracts = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.org.dash.platform.dapi.v0.GetProofsRequest.ContractRequest, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.clearContractsList = function() {
  return this.setContractsList([]);
};


/**
 * repeated DocumentRequest documents = 3;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest>}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.getDocumentsList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest, 3));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.setDocumentsList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 3, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest}
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.addDocuments = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 3, opt_value, proto.org.dash.platform.dapi.v0.GetProofsRequest.DocumentRequest, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsRequest.prototype.clearDocumentsList = function() {
  return this.setDocumentsList([]);
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
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.setProof = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional ResponseMetadata metadata = 2;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetProofsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetProofsResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 2) != null;
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
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.getId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  DATA_CONTRACT: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_[0]));
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
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getDataContract = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes data_contract = 1;
 * This is a type-conversion wrapper around `getDataContract()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getDataContract_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getDataContract_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getDataContract()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.setDataContract = function(value) {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.clearDataContract = function() {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.hasDataContract = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDataContractResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.getIdsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes ids = 1;
 * This is a type-conversion wrapper around `getIdsList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.getIdsList_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.getIdsList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getIdsList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.setIdsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.addIds = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.clearIdsList = function() {
  return this.setIdsList([]);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  DATA_CONTRACTS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractsResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractsResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_[0]));
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
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue;
  return proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.prototype.getValue = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes value = 1;
 * This is a type-conversion wrapper around `getValue()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.prototype.getValue_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.prototype.getValue_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getValue()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.prototype.setValue = function(value) {
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
    key: msg.getKey_asB64(),
    value: (f = msg.getValue()) && proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.toObject(includeInstance, f)
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
      msg.setKey(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.deserializeBinaryFromReader);
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
      proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.serializeBinaryToWriter
    );
  }
};


/**
 * optional bytes key = 1;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getKey = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes key = 1;
 * This is a type-conversion wrapper around `getKey()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getKey_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getKey_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getKey()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.setKey = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional DataContractValue value = 2;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.getValue = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.setValue = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.clearValue = function() {
  return this.setValue(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.prototype.hasValue = function() {
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
 * optional DataContracts data_contracts = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.getDataContracts = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.setDataContracts = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.clearDataContracts = function() {
  return this.setDataContracts(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.hasDataContracts = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDataContractsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractsResponse.prototype.hasMetadata = function() {
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
    id: msg.getId_asB64(),
    limit: jspb.Message.getFieldWithDefault(msg, 2, 0),
    offset: jspb.Message.getFieldWithDefault(msg, 3, 0),
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
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.setId(value);
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setLimit(value);
      break;
    case 3:
      var value = /** @type {number} */ (reader.readUint32());
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
  f = message.getId_asU8();
  if (f.length > 0) {
    writer.writeBytes(
      1,
      f
    );
  }
  f = message.getLimit();
  if (f !== 0) {
    writer.writeUint32(
      2,
      f
    );
  }
  f = message.getOffset();
  if (f !== 0) {
    writer.writeUint32(
      3,
      f
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes id = 1;
 * This is a type-conversion wrapper around `getId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.setId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional uint32 limit = 2;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getLimit = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.setLimit = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * optional uint32 offset = 3;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getOffset = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 3, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.setOffset = function(value) {
  return jspb.Message.setProto3IntField(this, 3, value);
};


/**
 * optional uint64 start_at_ms = 4;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getStartAtMs = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 4, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.setStartAtMs = function(value) {
  return jspb.Message.setProto3IntField(this, 4, value);
};


/**
 * optional bool prove = 5;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 5, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 5, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  DATA_CONTRACT_HISTORY: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_[0]));
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
    dataContractHistory: (f = msg.getDataContractHistory()) && proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.toObject(includeInstance, f),
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
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.deserializeBinaryFromReader);
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
  f = message.getDataContractHistory();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.serializeBinaryToWriter
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.getDate = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.setDate = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional bytes value = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.getValue = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * optional bytes value = 2;
 * This is a type-conversion wrapper around `getValue()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.getValue_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.getValue_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getValue()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.prototype.setValue = function(value) {
  return jspb.Message.setProto3BytesField(this, 2, value);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.toObject = function(includeInstance, msg) {
  var f, obj = {
    dataContractEntriesList: jspb.Message.toObjectList(msg.getDataContractEntriesList(),
    proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.toObject, includeInstance)
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
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory;
  return proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = new proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.deserializeBinaryFromReader);
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
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getDataContractEntriesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry.serializeBinaryToWriter
    );
  }
};


/**
 * repeated DataContractHistoryEntry data_contract_entries = 1;
 * @return {!Array<!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry>}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.prototype.getDataContractEntriesList = function() {
  return /** @type{!Array<!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry, 1));
};


/**
 * @param {!Array<!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry>} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.prototype.setDataContractEntriesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 1, value);
};


/**
 * @param {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry=} opt_value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.prototype.addDataContractEntries = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 1, opt_value, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistoryEntry, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory.prototype.clearDataContractEntriesList = function() {
  return this.setDataContractEntriesList([]);
};


/**
 * optional DataContractHistory data_contract_history = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.getDataContractHistory = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.DataContractHistory|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.setDataContractHistory = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.clearDataContractHistory = function() {
  return this.setDataContractHistory(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.hasDataContractHistory = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDataContractHistoryResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_ = [[6,7]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.StartCase = {
  START_NOT_SET: 0,
  START_AFTER: 6,
  START_AT: 7
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDocumentsRequest.StartCase}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getStartCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDocumentsRequest.StartCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_[0]));
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
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getDataContractId = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes data_contract_id = 1;
 * This is a type-conversion wrapper around `getDataContractId()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getDataContractId_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getDataContractId_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getDataContractId()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setDataContractId = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional string document_type = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getDocumentType = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setDocumentType = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional bytes where = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getWhere = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * optional bytes where = 3;
 * This is a type-conversion wrapper around `getWhere()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getWhere_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getWhere_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getWhere()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setWhere = function(value) {
  return jspb.Message.setProto3BytesField(this, 3, value);
};


/**
 * optional bytes order_by = 4;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getOrderBy = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 4, ""));
};


/**
 * optional bytes order_by = 4;
 * This is a type-conversion wrapper around `getOrderBy()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getOrderBy_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getOrderBy_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getOrderBy()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setOrderBy = function(value) {
  return jspb.Message.setProto3BytesField(this, 4, value);
};


/**
 * optional uint32 limit = 5;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getLimit = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 5, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setLimit = function(value) {
  return jspb.Message.setProto3IntField(this, 5, value);
};


/**
 * optional bytes start_after = 6;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getStartAfter = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 6, ""));
};


/**
 * optional bytes start_after = 6;
 * This is a type-conversion wrapper around `getStartAfter()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getStartAfter_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getStartAfter_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStartAfter()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setStartAfter = function(value) {
  return jspb.Message.setOneofField(this, 6, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.clearStartAfter = function() {
  return jspb.Message.setOneofField(this, 6, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.hasStartAfter = function() {
  return jspb.Message.getField(this, 6) != null;
};


/**
 * optional bytes start_at = 7;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getStartAt = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 7, ""));
};


/**
 * optional bytes start_at = 7;
 * This is a type-conversion wrapper around `getStartAt()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getStartAt_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getStartAt_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStartAt()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setStartAt = function(value) {
  return jspb.Message.setOneofField(this, 7, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.clearStartAt = function() {
  return jspb.Message.setOneofField(this, 7, proto.org.dash.platform.dapi.v0.GetDocumentsRequest.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.hasStartAt = function() {
  return jspb.Message.getField(this, 7) != null;
};


/**
 * optional bool prove = 8;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 8, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 8, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  DOCUMENTS: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetDocumentsResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetDocumentsResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_[0]));
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
    documents: (f = msg.getDocuments()) && proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.toObject(includeInstance, f),
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
      var value = new proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.deserializeBinaryFromReader);
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
  f = message.getDocuments();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.serializeBinaryToWriter
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
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents;
  return proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.getDocumentsList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes documents = 1;
 * This is a type-conversion wrapper around `getDocumentsList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.getDocumentsList_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.getDocumentsList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getDocumentsList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.setDocumentsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.addDocuments = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents.prototype.clearDocumentsList = function() {
  return this.setDocumentsList([]);
};


/**
 * optional Documents documents = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.getDocuments = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetDocumentsResponse.Documents|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.setDocuments = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.clearDocuments = function() {
  return this.setDocuments(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.hasDocuments = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetDocumentsResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetDocumentsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetDocumentsResponse.prototype.hasMetadata = function() {
  return jspb.Message.getField(this, 3) != null;
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.getPublicKeyHashesList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes public_key_hashes = 1;
 * This is a type-conversion wrapper around `getPublicKeyHashesList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.getPublicKeyHashesList_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.getPublicKeyHashesList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getPublicKeyHashesList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.setPublicKeyHashesList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.addPublicKeyHashes = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.clearPublicKeyHashesList = function() {
  return this.setPublicKeyHashesList([]);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITIES: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_[0]));
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
    identities: (f = msg.getIdentities()) && proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.toObject(includeInstance, f),
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
      var value = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.deserializeBinaryFromReader);
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
  f = message.getIdentities();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.serializeBinaryToWriter
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
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.repeatedFields_ = [1];



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
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.toObject = function(includeInstance, msg) {
  var f, obj = {
    identitiesList: msg.getIdentitiesList_asB64()
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities;
  return proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {!Uint8Array} */ (reader.readBytes());
      msg.addIdentities(value);
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
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getIdentitiesList_asU8();
  if (f.length > 0) {
    writer.writeRepeatedBytes(
      1,
      f
    );
  }
};


/**
 * repeated bytes identities = 1;
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.getIdentitiesList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * repeated bytes identities = 1;
 * This is a type-conversion wrapper around `getIdentitiesList()`
 * @return {!Array<string>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.getIdentitiesList_asB64 = function() {
  return /** @type {!Array<string>} */ (jspb.Message.bytesListAsB64(
      this.getIdentitiesList()));
};


/**
 * repeated bytes identities = 1;
 * Note that Uint8Array is not supported on all browsers.
 * @see http://caniuse.com/Uint8Array
 * This is a type-conversion wrapper around `getIdentitiesList()`
 * @return {!Array<!Uint8Array>}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.getIdentitiesList_asU8 = function() {
  return /** @type {!Array<!Uint8Array>} */ (jspb.Message.bytesListAsU8(
      this.getIdentitiesList()));
};


/**
 * @param {!(Array<!Uint8Array>|Array<string>)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.setIdentitiesList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {!(string|Uint8Array)} value
 * @param {number=} opt_index
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.addIdentities = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities.prototype.clearIdentitiesList = function() {
  return this.setIdentitiesList([]);
};


/**
 * optional Identities identities = 1;
 * @return {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.getIdentities = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.Identities|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.setIdentities = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.clearIdentities = function() {
  return this.setIdentities(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.hasIdentities = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.prototype.hasMetadata = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest;
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.getPublicKeyHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes public_key_hash = 1;
 * This is a type-conversion wrapper around `getPublicKeyHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.getPublicKeyHash_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.getPublicKeyHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getPublicKeyHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.setPublicKeyHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  IDENTITY: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.oneofGroups_[0]));
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse;
  return proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.getIdentity = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes identity = 1;
 * This is a type-conversion wrapper around `getIdentity()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.getIdentity_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.getIdentity_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getIdentity()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.setIdentity = function(value) {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.oneofGroups_[0], value);
};


/**
 * Clears the field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.clearIdentity = function() {
  return jspb.Message.setOneofField(this, 1, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.oneofGroups_[0], undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.hasIdentity = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.prototype.hasMetadata = function() {
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
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.getStateTransitionHash = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * optional bytes state_transition_hash = 1;
 * This is a type-conversion wrapper around `getStateTransitionHash()`
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.getStateTransitionHash_asB64 = function() {
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
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.getStateTransitionHash_asU8 = function() {
  return /** @type {!Uint8Array} */ (jspb.Message.bytesAsU8(
      this.getStateTransitionHash()));
};


/**
 * @param {!(string|Uint8Array)} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.setStateTransitionHash = function(value) {
  return jspb.Message.setProto3BytesField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
};



/**
 * Oneof group definitions for this message. Each group defines the field
 * numbers belonging to that group. When of these fields' value is set, all
 * other fields in the group are cleared. During deserialization, if multiple
 * fields are encountered for a group, only the last value seen will be kept.
 * @private {!Array<!Array<number>>}
 * @const
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_ = [[1,2]];

/**
 * @enum {number}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.ResultCase = {
  RESULT_NOT_SET: 0,
  ERROR: 1,
  PROOF: 2
};

/**
 * @return {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.ResultCase}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.getResultCase = function() {
  return /** @type {proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.ResultCase} */(jspb.Message.computeOneofCase(this, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_[0]));
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
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.getError = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.StateTransitionBroadcastError|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.setError = function(value) {
  return jspb.Message.setOneofWrapperField(this, 1, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.clearError = function() {
  return this.setError(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.hasError = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional Proof proof = 2;
 * @return {?proto.org.dash.platform.dapi.v0.Proof}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.getProof = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.Proof} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.Proof, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.Proof|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.setProof = function(value) {
  return jspb.Message.setOneofWrapperField(this, 2, proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.oneofGroups_[0], value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.clearProof = function() {
  return this.setProof(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.hasProof = function() {
  return jspb.Message.getField(this, 2) != null;
};


/**
 * optional ResponseMetadata metadata = 3;
 * @return {?proto.org.dash.platform.dapi.v0.ResponseMetadata}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.getMetadata = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ResponseMetadata} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ResponseMetadata, 3));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ResponseMetadata|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
*/
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.setMetadata = function(value) {
  return jspb.Message.setWrapperField(this, 3, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} returns this
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.clearMetadata = function() {
  return this.setMetadata(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.prototype.hasMetadata = function() {
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
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.ConsensusParamsBlock;
  return proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.getMaxBytes = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock} returns this
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.setMaxBytes = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string max_gas = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.getMaxGas = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock} returns this
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.setMaxGas = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string time_iota_ms = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.getTimeIotaMs = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsBlock} returns this
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.prototype.setTimeIotaMs = function(value) {
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
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.toObject = function(opt_includeInstance) {
  return proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.toObject = function(includeInstance, msg) {
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
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence;
  return proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.deserializeBinaryFromReader = function(msg, reader) {
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
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.serializeBinaryToWriter = function(message, writer) {
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
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.getMaxAgeNumBlocks = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 1, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence} returns this
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.setMaxAgeNumBlocks = function(value) {
  return jspb.Message.setProto3StringField(this, 1, value);
};


/**
 * optional string max_age_duration = 2;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.getMaxAgeDuration = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 2, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence} returns this
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.setMaxAgeDuration = function(value) {
  return jspb.Message.setProto3StringField(this, 2, value);
};


/**
 * optional string max_bytes = 3;
 * @return {string}
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.getMaxBytes = function() {
  return /** @type {string} */ (jspb.Message.getFieldWithDefault(this, 3, ""));
};


/**
 * @param {string} value
 * @return {!proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence} returns this
 */
proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.prototype.setMaxBytes = function(value) {
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
      var value = /** @type {number} */ (reader.readInt64());
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
  f = message.getHeight();
  if (f !== 0) {
    writer.writeInt64(
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
 * optional int64 height = 1;
 * @return {number}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.getHeight = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.setHeight = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * optional bool prove = 2;
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.getProve = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 2, false));
};


/**
 * @param {boolean} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsRequest.prototype.setProve = function(value) {
  return jspb.Message.setProto3BooleanField(this, 2, value);
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
    block: (f = msg.getBlock()) && proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.toObject(includeInstance, f),
    evidence: (f = msg.getEvidence()) && proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.toObject(includeInstance, f)
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
      var value = new proto.org.dash.platform.dapi.v0.ConsensusParamsBlock;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.deserializeBinaryFromReader);
      msg.setBlock(value);
      break;
    case 2:
      var value = new proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence;
      reader.readMessage(value,proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.deserializeBinaryFromReader);
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
  f = message.getBlock();
  if (f != null) {
    writer.writeMessage(
      1,
      f,
      proto.org.dash.platform.dapi.v0.ConsensusParamsBlock.serializeBinaryToWriter
    );
  }
  f = message.getEvidence();
  if (f != null) {
    writer.writeMessage(
      2,
      f,
      proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence.serializeBinaryToWriter
    );
  }
};


/**
 * optional ConsensusParamsBlock block = 1;
 * @return {?proto.org.dash.platform.dapi.v0.ConsensusParamsBlock}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.getBlock = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ConsensusParamsBlock} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ConsensusParamsBlock, 1));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ConsensusParamsBlock|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.setBlock = function(value) {
  return jspb.Message.setWrapperField(this, 1, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.clearBlock = function() {
  return this.setBlock(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.hasBlock = function() {
  return jspb.Message.getField(this, 1) != null;
};


/**
 * optional ConsensusParamsEvidence evidence = 2;
 * @return {?proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.getEvidence = function() {
  return /** @type{?proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence} */ (
    jspb.Message.getWrapperField(this, proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence, 2));
};


/**
 * @param {?proto.org.dash.platform.dapi.v0.ConsensusParamsEvidence|undefined} value
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} returns this
*/
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.setEvidence = function(value) {
  return jspb.Message.setWrapperField(this, 2, value);
};


/**
 * Clears the message field making it undefined.
 * @return {!proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse} returns this
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.clearEvidence = function() {
  return this.setEvidence(undefined);
};


/**
 * Returns whether this field is set.
 * @return {boolean}
 */
proto.org.dash.platform.dapi.v0.GetConsensusParamsResponse.prototype.hasEvidence = function() {
  return jspb.Message.getField(this, 2) != null;
};


goog.object.extend(exports, proto.org.dash.platform.dapi.v0);
