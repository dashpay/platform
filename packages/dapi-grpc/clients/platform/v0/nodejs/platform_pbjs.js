/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars*/
"use strict";

var $protobuf = require("@dashevo/protobufjs/minimal");

// Common aliases
var $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;

// Exported root namespace
var $root = $protobuf.roots.platform_root || ($protobuf.roots.platform_root = {});

$root.org = (function() {

    /**
     * Namespace org.
     * @exports org
     * @namespace
     */
    var org = {};

    org.dash = (function() {

        /**
         * Namespace dash.
         * @memberof org
         * @namespace
         */
        var dash = {};

        dash.platform = (function() {

            /**
             * Namespace platform.
             * @memberof org.dash
             * @namespace
             */
            var platform = {};

            platform.dapi = (function() {

                /**
                 * Namespace dapi.
                 * @memberof org.dash.platform
                 * @namespace
                 */
                var dapi = {};

                dapi.v0 = (function() {

                    /**
                     * Namespace v0.
                     * @memberof org.dash.platform.dapi
                     * @namespace
                     */
                    var v0 = {};

                    v0.Platform = (function() {

                        /**
                         * Constructs a new Platform service.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a Platform
                         * @extends $protobuf.rpc.Service
                         * @constructor
                         * @param {$protobuf.RPCImpl} rpcImpl RPC implementation
                         * @param {boolean} [requestDelimited=false] Whether requests are length-delimited
                         * @param {boolean} [responseDelimited=false] Whether responses are length-delimited
                         */
                        function Platform(rpcImpl, requestDelimited, responseDelimited) {
                            $protobuf.rpc.Service.call(this, rpcImpl, requestDelimited, responseDelimited);
                        }

                        (Platform.prototype = Object.create($protobuf.rpc.Service.prototype)).constructor = Platform;

                        /**
                         * Creates new Platform service using the specified rpc implementation.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @static
                         * @param {$protobuf.RPCImpl} rpcImpl RPC implementation
                         * @param {boolean} [requestDelimited=false] Whether requests are length-delimited
                         * @param {boolean} [responseDelimited=false] Whether responses are length-delimited
                         * @returns {Platform} RPC service. Useful where requests and/or responses are streamed.
                         */
                        Platform.create = function create(rpcImpl, requestDelimited, responseDelimited) {
                            return new this(rpcImpl, requestDelimited, responseDelimited);
                        };

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#broadcastStateTransition}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef broadcastStateTransitionCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} [response] BroadcastStateTransitionResponse
                         */

                        /**
                         * Calls broadcastStateTransition.
                         * @function broadcastStateTransition
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} request BroadcastStateTransitionRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.broadcastStateTransitionCallback} callback Node-style callback called with the error, if any, and BroadcastStateTransitionResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.broadcastStateTransition = function broadcastStateTransition(request, callback) {
                            return this.rpcCall(broadcastStateTransition, $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest, $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse, request, callback);
                        }, "name", { value: "broadcastStateTransition" });

                        /**
                         * Calls broadcastStateTransition.
                         * @function broadcastStateTransition
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} request BroadcastStateTransitionRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.BroadcastStateTransitionResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentity}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetIdentityResponse} [response] GetIdentityResponse
                         */

                        /**
                         * Calls getIdentity.
                         * @function getIdentity
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} request GetIdentityRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityCallback} callback Node-style callback called with the error, if any, and GetIdentityResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentity = function getIdentity(request, callback) {
                            return this.rpcCall(getIdentity, $root.org.dash.platform.dapi.v0.GetIdentityRequest, $root.org.dash.platform.dapi.v0.GetIdentityResponse, request, callback);
                        }, "name", { value: "getIdentity" });

                        /**
                         * Calls getIdentity.
                         * @function getIdentity
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} request GetIdentityRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetIdentityResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentityKeys}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityKeysCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetIdentityKeysResponse} [response] GetIdentityKeysResponse
                         */

                        /**
                         * Calls getIdentityKeys.
                         * @function getIdentityKeys
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysRequest} request GetIdentityKeysRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityKeysCallback} callback Node-style callback called with the error, if any, and GetIdentityKeysResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentityKeys = function getIdentityKeys(request, callback) {
                            return this.rpcCall(getIdentityKeys, $root.org.dash.platform.dapi.v0.GetIdentityKeysRequest, $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse, request, callback);
                        }, "name", { value: "getIdentityKeys" });

                        /**
                         * Calls getIdentityKeys.
                         * @function getIdentityKeys
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysRequest} request GetIdentityKeysRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetIdentityKeysResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentityBalance}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityBalanceCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetIdentityBalanceResponse} [response] GetIdentityBalanceResponse
                         */

                        /**
                         * Calls getIdentityBalance.
                         * @function getIdentityBalance
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} request GetIdentityRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityBalanceCallback} callback Node-style callback called with the error, if any, and GetIdentityBalanceResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentityBalance = function getIdentityBalance(request, callback) {
                            return this.rpcCall(getIdentityBalance, $root.org.dash.platform.dapi.v0.GetIdentityRequest, $root.org.dash.platform.dapi.v0.GetIdentityBalanceResponse, request, callback);
                        }, "name", { value: "getIdentityBalance" });

                        /**
                         * Calls getIdentityBalance.
                         * @function getIdentityBalance
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} request GetIdentityRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetIdentityBalanceResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentityBalanceAndRevision}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityBalanceAndRevisionCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} [response] GetIdentityBalanceAndRevisionResponse
                         */

                        /**
                         * Calls getIdentityBalanceAndRevision.
                         * @function getIdentityBalanceAndRevision
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} request GetIdentityRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityBalanceAndRevisionCallback} callback Node-style callback called with the error, if any, and GetIdentityBalanceAndRevisionResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentityBalanceAndRevision = function getIdentityBalanceAndRevision(request, callback) {
                            return this.rpcCall(getIdentityBalanceAndRevision, $root.org.dash.platform.dapi.v0.GetIdentityRequest, $root.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse, request, callback);
                        }, "name", { value: "getIdentityBalanceAndRevision" });

                        /**
                         * Calls getIdentityBalanceAndRevision.
                         * @function getIdentityBalanceAndRevision
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} request GetIdentityRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getDataContract}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getDataContractCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetDataContractResponse} [response] GetDataContractResponse
                         */

                        /**
                         * Calls getDataContract.
                         * @function getDataContract
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDataContractRequest} request GetDataContractRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getDataContractCallback} callback Node-style callback called with the error, if any, and GetDataContractResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getDataContract = function getDataContract(request, callback) {
                            return this.rpcCall(getDataContract, $root.org.dash.platform.dapi.v0.GetDataContractRequest, $root.org.dash.platform.dapi.v0.GetDataContractResponse, request, callback);
                        }, "name", { value: "getDataContract" });

                        /**
                         * Calls getDataContract.
                         * @function getDataContract
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDataContractRequest} request GetDataContractRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetDataContractResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getDataContracts}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getDataContractsCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetDataContractsResponse} [response] GetDataContractsResponse
                         */

                        /**
                         * Calls getDataContracts.
                         * @function getDataContracts
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsRequest} request GetDataContractsRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getDataContractsCallback} callback Node-style callback called with the error, if any, and GetDataContractsResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getDataContracts = function getDataContracts(request, callback) {
                            return this.rpcCall(getDataContracts, $root.org.dash.platform.dapi.v0.GetDataContractsRequest, $root.org.dash.platform.dapi.v0.GetDataContractsResponse, request, callback);
                        }, "name", { value: "getDataContracts" });

                        /**
                         * Calls getDataContracts.
                         * @function getDataContracts
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsRequest} request GetDataContractsRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetDataContractsResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getDocuments}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getDocumentsCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetDocumentsResponse} [response] GetDocumentsResponse
                         */

                        /**
                         * Calls getDocuments.
                         * @function getDocuments
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} request GetDocumentsRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getDocumentsCallback} callback Node-style callback called with the error, if any, and GetDocumentsResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getDocuments = function getDocuments(request, callback) {
                            return this.rpcCall(getDocuments, $root.org.dash.platform.dapi.v0.GetDocumentsRequest, $root.org.dash.platform.dapi.v0.GetDocumentsResponse, request, callback);
                        }, "name", { value: "getDocuments" });

                        /**
                         * Calls getDocuments.
                         * @function getDocuments
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} request GetDocumentsRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetDocumentsResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentitiesByPublicKeyHashes}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentitiesByPublicKeyHashesCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} [response] GetIdentitiesByPublicKeyHashesResponse
                         */

                        /**
                         * Calls getIdentitiesByPublicKeyHashes.
                         * @function getIdentitiesByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesRequest} request GetIdentitiesByPublicKeyHashesRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentitiesByPublicKeyHashesCallback} callback Node-style callback called with the error, if any, and GetIdentitiesByPublicKeyHashesResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentitiesByPublicKeyHashes = function getIdentitiesByPublicKeyHashes(request, callback) {
                            return this.rpcCall(getIdentitiesByPublicKeyHashes, $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest, $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse, request, callback);
                        }, "name", { value: "getIdentitiesByPublicKeyHashes" });

                        /**
                         * Calls getIdentitiesByPublicKeyHashes.
                         * @function getIdentitiesByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesRequest} request GetIdentitiesByPublicKeyHashesRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentityByPublicKeyHashes}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityByPublicKeyHashesCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} [response] GetIdentityByPublicKeyHashesResponse
                         */

                        /**
                         * Calls getIdentityByPublicKeyHashes.
                         * @function getIdentityByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesRequest} request GetIdentityByPublicKeyHashesRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityByPublicKeyHashesCallback} callback Node-style callback called with the error, if any, and GetIdentityByPublicKeyHashesResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentityByPublicKeyHashes = function getIdentityByPublicKeyHashes(request, callback) {
                            return this.rpcCall(getIdentityByPublicKeyHashes, $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest, $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse, request, callback);
                        }, "name", { value: "getIdentityByPublicKeyHashes" });

                        /**
                         * Calls getIdentityByPublicKeyHashes.
                         * @function getIdentityByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesRequest} request GetIdentityByPublicKeyHashesRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#waitForStateTransitionResult}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef waitForStateTransitionResultCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} [response] WaitForStateTransitionResultResponse
                         */

                        /**
                         * Calls waitForStateTransitionResult.
                         * @function waitForStateTransitionResult
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} request WaitForStateTransitionResultRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.waitForStateTransitionResultCallback} callback Node-style callback called with the error, if any, and WaitForStateTransitionResultResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.waitForStateTransitionResult = function waitForStateTransitionResult(request, callback) {
                            return this.rpcCall(waitForStateTransitionResult, $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest, $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse, request, callback);
                        }, "name", { value: "waitForStateTransitionResult" });

                        /**
                         * Calls waitForStateTransitionResult.
                         * @function waitForStateTransitionResult
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} request WaitForStateTransitionResultRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getConsensusParams}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getConsensusParamsCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetConsensusParamsResponse} [response] GetConsensusParamsResponse
                         */

                        /**
                         * Calls getConsensusParams.
                         * @function getConsensusParams
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} request GetConsensusParamsRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getConsensusParamsCallback} callback Node-style callback called with the error, if any, and GetConsensusParamsResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getConsensusParams = function getConsensusParams(request, callback) {
                            return this.rpcCall(getConsensusParams, $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest, $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse, request, callback);
                        }, "name", { value: "getConsensusParams" });

                        /**
                         * Calls getConsensusParams.
                         * @function getConsensusParams
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} request GetConsensusParamsRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetConsensusParamsResponse>} Promise
                         * @variation 2
                         */

                        return Platform;
                    })();

                    v0.Proof = (function() {

                        /**
                         * Properties of a Proof.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IProof
                         * @property {Uint8Array|null} [grovedbProof] Proof grovedbProof
                         * @property {Uint8Array|null} [quorumHash] Proof quorumHash
                         * @property {Uint8Array|null} [signature] Proof signature
                         * @property {number|null} [round] Proof round
                         */

                        /**
                         * Constructs a new Proof.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a Proof.
                         * @implements IProof
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IProof=} [properties] Properties to set
                         */
                        function Proof(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * Proof grovedbProof.
                         * @member {Uint8Array} grovedbProof
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.grovedbProof = $util.newBuffer([]);

                        /**
                         * Proof quorumHash.
                         * @member {Uint8Array} quorumHash
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.quorumHash = $util.newBuffer([]);

                        /**
                         * Proof signature.
                         * @member {Uint8Array} signature
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.signature = $util.newBuffer([]);

                        /**
                         * Proof round.
                         * @member {number} round
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.round = 0;

                        /**
                         * Creates a new Proof instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {org.dash.platform.dapi.v0.IProof=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.Proof} Proof instance
                         */
                        Proof.create = function create(properties) {
                            return new Proof(properties);
                        };

                        /**
                         * Encodes the specified Proof message. Does not implicitly {@link org.dash.platform.dapi.v0.Proof.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {org.dash.platform.dapi.v0.IProof} message Proof message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        Proof.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.grovedbProof != null && Object.hasOwnProperty.call(message, "grovedbProof"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.grovedbProof);
                            if (message.quorumHash != null && Object.hasOwnProperty.call(message, "quorumHash"))
                                writer.uint32(/* id 2, wireType 2 =*/18).bytes(message.quorumHash);
                            if (message.signature != null && Object.hasOwnProperty.call(message, "signature"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.signature);
                            if (message.round != null && Object.hasOwnProperty.call(message, "round"))
                                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.round);
                            return writer;
                        };

                        /**
                         * Encodes the specified Proof message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.Proof.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {org.dash.platform.dapi.v0.IProof} message Proof message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        Proof.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a Proof message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.Proof} Proof
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        Proof.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.Proof();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.grovedbProof = reader.bytes();
                                    break;
                                case 2:
                                    message.quorumHash = reader.bytes();
                                    break;
                                case 3:
                                    message.signature = reader.bytes();
                                    break;
                                case 4:
                                    message.round = reader.uint32();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a Proof message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.Proof} Proof
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        Proof.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a Proof message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        Proof.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.grovedbProof != null && message.hasOwnProperty("grovedbProof"))
                                if (!(message.grovedbProof && typeof message.grovedbProof.length === "number" || $util.isString(message.grovedbProof)))
                                    return "grovedbProof: buffer expected";
                            if (message.quorumHash != null && message.hasOwnProperty("quorumHash"))
                                if (!(message.quorumHash && typeof message.quorumHash.length === "number" || $util.isString(message.quorumHash)))
                                    return "quorumHash: buffer expected";
                            if (message.signature != null && message.hasOwnProperty("signature"))
                                if (!(message.signature && typeof message.signature.length === "number" || $util.isString(message.signature)))
                                    return "signature: buffer expected";
                            if (message.round != null && message.hasOwnProperty("round"))
                                if (!$util.isInteger(message.round))
                                    return "round: integer expected";
                            return null;
                        };

                        /**
                         * Creates a Proof message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.Proof} Proof
                         */
                        Proof.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.Proof)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.Proof();
                            if (object.grovedbProof != null)
                                if (typeof object.grovedbProof === "string")
                                    $util.base64.decode(object.grovedbProof, message.grovedbProof = $util.newBuffer($util.base64.length(object.grovedbProof)), 0);
                                else if (object.grovedbProof.length >= 0)
                                    message.grovedbProof = object.grovedbProof;
                            if (object.quorumHash != null)
                                if (typeof object.quorumHash === "string")
                                    $util.base64.decode(object.quorumHash, message.quorumHash = $util.newBuffer($util.base64.length(object.quorumHash)), 0);
                                else if (object.quorumHash.length >= 0)
                                    message.quorumHash = object.quorumHash;
                            if (object.signature != null)
                                if (typeof object.signature === "string")
                                    $util.base64.decode(object.signature, message.signature = $util.newBuffer($util.base64.length(object.signature)), 0);
                                else if (object.signature.length >= 0)
                                    message.signature = object.signature;
                            if (object.round != null)
                                message.round = object.round >>> 0;
                            return message;
                        };

                        /**
                         * Creates a plain object from a Proof message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @static
                         * @param {org.dash.platform.dapi.v0.Proof} message Proof
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        Proof.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.grovedbProof = "";
                                else {
                                    object.grovedbProof = [];
                                    if (options.bytes !== Array)
                                        object.grovedbProof = $util.newBuffer(object.grovedbProof);
                                }
                                if (options.bytes === String)
                                    object.quorumHash = "";
                                else {
                                    object.quorumHash = [];
                                    if (options.bytes !== Array)
                                        object.quorumHash = $util.newBuffer(object.quorumHash);
                                }
                                if (options.bytes === String)
                                    object.signature = "";
                                else {
                                    object.signature = [];
                                    if (options.bytes !== Array)
                                        object.signature = $util.newBuffer(object.signature);
                                }
                                object.round = 0;
                            }
                            if (message.grovedbProof != null && message.hasOwnProperty("grovedbProof"))
                                object.grovedbProof = options.bytes === String ? $util.base64.encode(message.grovedbProof, 0, message.grovedbProof.length) : options.bytes === Array ? Array.prototype.slice.call(message.grovedbProof) : message.grovedbProof;
                            if (message.quorumHash != null && message.hasOwnProperty("quorumHash"))
                                object.quorumHash = options.bytes === String ? $util.base64.encode(message.quorumHash, 0, message.quorumHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.quorumHash) : message.quorumHash;
                            if (message.signature != null && message.hasOwnProperty("signature"))
                                object.signature = options.bytes === String ? $util.base64.encode(message.signature, 0, message.signature.length) : options.bytes === Array ? Array.prototype.slice.call(message.signature) : message.signature;
                            if (message.round != null && message.hasOwnProperty("round"))
                                object.round = message.round;
                            return object;
                        };

                        /**
                         * Converts this Proof to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        Proof.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return Proof;
                    })();

                    v0.ResponseMetadata = (function() {

                        /**
                         * Properties of a ResponseMetadata.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IResponseMetadata
                         * @property {number|Long|null} [height] ResponseMetadata height
                         * @property {number|null} [coreChainLockedHeight] ResponseMetadata coreChainLockedHeight
                         * @property {number|Long|null} [timeMs] ResponseMetadata timeMs
                         * @property {number|null} [protocolVersion] ResponseMetadata protocolVersion
                         */

                        /**
                         * Constructs a new ResponseMetadata.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ResponseMetadata.
                         * @implements IResponseMetadata
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata=} [properties] Properties to set
                         */
                        function ResponseMetadata(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ResponseMetadata height.
                         * @member {number|Long} height
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.height = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                        /**
                         * ResponseMetadata coreChainLockedHeight.
                         * @member {number} coreChainLockedHeight
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.coreChainLockedHeight = 0;

                        /**
                         * ResponseMetadata timeMs.
                         * @member {number|Long} timeMs
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.timeMs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                        /**
                         * ResponseMetadata protocolVersion.
                         * @member {number} protocolVersion
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.protocolVersion = 0;

                        /**
                         * Creates a new ResponseMetadata instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata instance
                         */
                        ResponseMetadata.create = function create(properties) {
                            return new ResponseMetadata(properties);
                        };

                        /**
                         * Encodes the specified ResponseMetadata message. Does not implicitly {@link org.dash.platform.dapi.v0.ResponseMetadata.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata} message ResponseMetadata message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ResponseMetadata.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.height != null && Object.hasOwnProperty.call(message, "height"))
                                writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.height);
                            if (message.coreChainLockedHeight != null && Object.hasOwnProperty.call(message, "coreChainLockedHeight"))
                                writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.coreChainLockedHeight);
                            if (message.timeMs != null && Object.hasOwnProperty.call(message, "timeMs"))
                                writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.timeMs);
                            if (message.protocolVersion != null && Object.hasOwnProperty.call(message, "protocolVersion"))
                                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.protocolVersion);
                            return writer;
                        };

                        /**
                         * Encodes the specified ResponseMetadata message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ResponseMetadata.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata} message ResponseMetadata message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ResponseMetadata.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ResponseMetadata message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ResponseMetadata.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ResponseMetadata();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.height = reader.uint64();
                                    break;
                                case 2:
                                    message.coreChainLockedHeight = reader.uint32();
                                    break;
                                case 3:
                                    message.timeMs = reader.uint64();
                                    break;
                                case 4:
                                    message.protocolVersion = reader.uint32();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ResponseMetadata message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ResponseMetadata.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ResponseMetadata message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ResponseMetadata.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (!$util.isInteger(message.height) && !(message.height && $util.isInteger(message.height.low) && $util.isInteger(message.height.high)))
                                    return "height: integer|Long expected";
                            if (message.coreChainLockedHeight != null && message.hasOwnProperty("coreChainLockedHeight"))
                                if (!$util.isInteger(message.coreChainLockedHeight))
                                    return "coreChainLockedHeight: integer expected";
                            if (message.timeMs != null && message.hasOwnProperty("timeMs"))
                                if (!$util.isInteger(message.timeMs) && !(message.timeMs && $util.isInteger(message.timeMs.low) && $util.isInteger(message.timeMs.high)))
                                    return "timeMs: integer|Long expected";
                            if (message.protocolVersion != null && message.hasOwnProperty("protocolVersion"))
                                if (!$util.isInteger(message.protocolVersion))
                                    return "protocolVersion: integer expected";
                            return null;
                        };

                        /**
                         * Creates a ResponseMetadata message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata
                         */
                        ResponseMetadata.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ResponseMetadata)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ResponseMetadata();
                            if (object.height != null)
                                if ($util.Long)
                                    (message.height = $util.Long.fromValue(object.height)).unsigned = true;
                                else if (typeof object.height === "string")
                                    message.height = parseInt(object.height, 10);
                                else if (typeof object.height === "number")
                                    message.height = object.height;
                                else if (typeof object.height === "object")
                                    message.height = new $util.LongBits(object.height.low >>> 0, object.height.high >>> 0).toNumber(true);
                            if (object.coreChainLockedHeight != null)
                                message.coreChainLockedHeight = object.coreChainLockedHeight >>> 0;
                            if (object.timeMs != null)
                                if ($util.Long)
                                    (message.timeMs = $util.Long.fromValue(object.timeMs)).unsigned = true;
                                else if (typeof object.timeMs === "string")
                                    message.timeMs = parseInt(object.timeMs, 10);
                                else if (typeof object.timeMs === "number")
                                    message.timeMs = object.timeMs;
                                else if (typeof object.timeMs === "object")
                                    message.timeMs = new $util.LongBits(object.timeMs.low >>> 0, object.timeMs.high >>> 0).toNumber(true);
                            if (object.protocolVersion != null)
                                message.protocolVersion = object.protocolVersion >>> 0;
                            return message;
                        };

                        /**
                         * Creates a plain object from a ResponseMetadata message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.ResponseMetadata} message ResponseMetadata
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ResponseMetadata.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if ($util.Long) {
                                    var long = new $util.Long(0, 0, true);
                                    object.height = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                                } else
                                    object.height = options.longs === String ? "0" : 0;
                                object.coreChainLockedHeight = 0;
                                if ($util.Long) {
                                    var long = new $util.Long(0, 0, true);
                                    object.timeMs = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                                } else
                                    object.timeMs = options.longs === String ? "0" : 0;
                                object.protocolVersion = 0;
                            }
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (typeof message.height === "number")
                                    object.height = options.longs === String ? String(message.height) : message.height;
                                else
                                    object.height = options.longs === String ? $util.Long.prototype.toString.call(message.height) : options.longs === Number ? new $util.LongBits(message.height.low >>> 0, message.height.high >>> 0).toNumber(true) : message.height;
                            if (message.coreChainLockedHeight != null && message.hasOwnProperty("coreChainLockedHeight"))
                                object.coreChainLockedHeight = message.coreChainLockedHeight;
                            if (message.timeMs != null && message.hasOwnProperty("timeMs"))
                                if (typeof message.timeMs === "number")
                                    object.timeMs = options.longs === String ? String(message.timeMs) : message.timeMs;
                                else
                                    object.timeMs = options.longs === String ? $util.Long.prototype.toString.call(message.timeMs) : options.longs === Number ? new $util.LongBits(message.timeMs.low >>> 0, message.timeMs.high >>> 0).toNumber(true) : message.timeMs;
                            if (message.protocolVersion != null && message.hasOwnProperty("protocolVersion"))
                                object.protocolVersion = message.protocolVersion;
                            return object;
                        };

                        /**
                         * Converts this ResponseMetadata to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ResponseMetadata.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ResponseMetadata;
                    })();

                    v0.StateTransitionBroadcastError = (function() {

                        /**
                         * Properties of a StateTransitionBroadcastError.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IStateTransitionBroadcastError
                         * @property {number|null} [code] StateTransitionBroadcastError code
                         * @property {string|null} [message] StateTransitionBroadcastError message
                         * @property {Uint8Array|null} [data] StateTransitionBroadcastError data
                         */

                        /**
                         * Constructs a new StateTransitionBroadcastError.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a StateTransitionBroadcastError.
                         * @implements IStateTransitionBroadcastError
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError=} [properties] Properties to set
                         */
                        function StateTransitionBroadcastError(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * StateTransitionBroadcastError code.
                         * @member {number} code
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         */
                        StateTransitionBroadcastError.prototype.code = 0;

                        /**
                         * StateTransitionBroadcastError message.
                         * @member {string} message
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         */
                        StateTransitionBroadcastError.prototype.message = "";

                        /**
                         * StateTransitionBroadcastError data.
                         * @member {Uint8Array} data
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         */
                        StateTransitionBroadcastError.prototype.data = $util.newBuffer([]);

                        /**
                         * Creates a new StateTransitionBroadcastError instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError instance
                         */
                        StateTransitionBroadcastError.create = function create(properties) {
                            return new StateTransitionBroadcastError(properties);
                        };

                        /**
                         * Encodes the specified StateTransitionBroadcastError message. Does not implicitly {@link org.dash.platform.dapi.v0.StateTransitionBroadcastError.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError} message StateTransitionBroadcastError message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        StateTransitionBroadcastError.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.code != null && Object.hasOwnProperty.call(message, "code"))
                                writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.code);
                            if (message.message != null && Object.hasOwnProperty.call(message, "message"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.message);
                            if (message.data != null && Object.hasOwnProperty.call(message, "data"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.data);
                            return writer;
                        };

                        /**
                         * Encodes the specified StateTransitionBroadcastError message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.StateTransitionBroadcastError.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError} message StateTransitionBroadcastError message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        StateTransitionBroadcastError.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a StateTransitionBroadcastError message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        StateTransitionBroadcastError.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.code = reader.uint32();
                                    break;
                                case 2:
                                    message.message = reader.string();
                                    break;
                                case 3:
                                    message.data = reader.bytes();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a StateTransitionBroadcastError message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        StateTransitionBroadcastError.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a StateTransitionBroadcastError message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        StateTransitionBroadcastError.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.code != null && message.hasOwnProperty("code"))
                                if (!$util.isInteger(message.code))
                                    return "code: integer expected";
                            if (message.message != null && message.hasOwnProperty("message"))
                                if (!$util.isString(message.message))
                                    return "message: string expected";
                            if (message.data != null && message.hasOwnProperty("data"))
                                if (!(message.data && typeof message.data.length === "number" || $util.isString(message.data)))
                                    return "data: buffer expected";
                            return null;
                        };

                        /**
                         * Creates a StateTransitionBroadcastError message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError
                         */
                        StateTransitionBroadcastError.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError();
                            if (object.code != null)
                                message.code = object.code >>> 0;
                            if (object.message != null)
                                message.message = String(object.message);
                            if (object.data != null)
                                if (typeof object.data === "string")
                                    $util.base64.decode(object.data, message.data = $util.newBuffer($util.base64.length(object.data)), 0);
                                else if (object.data.length >= 0)
                                    message.data = object.data;
                            return message;
                        };

                        /**
                         * Creates a plain object from a StateTransitionBroadcastError message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.StateTransitionBroadcastError} message StateTransitionBroadcastError
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        StateTransitionBroadcastError.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.code = 0;
                                object.message = "";
                                if (options.bytes === String)
                                    object.data = "";
                                else {
                                    object.data = [];
                                    if (options.bytes !== Array)
                                        object.data = $util.newBuffer(object.data);
                                }
                            }
                            if (message.code != null && message.hasOwnProperty("code"))
                                object.code = message.code;
                            if (message.message != null && message.hasOwnProperty("message"))
                                object.message = message.message;
                            if (message.data != null && message.hasOwnProperty("data"))
                                object.data = options.bytes === String ? $util.base64.encode(message.data, 0, message.data.length) : options.bytes === Array ? Array.prototype.slice.call(message.data) : message.data;
                            return object;
                        };

                        /**
                         * Converts this StateTransitionBroadcastError to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        StateTransitionBroadcastError.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return StateTransitionBroadcastError;
                    })();

                    v0.BroadcastStateTransitionRequest = (function() {

                        /**
                         * Properties of a BroadcastStateTransitionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBroadcastStateTransitionRequest
                         * @property {Uint8Array|null} [stateTransition] BroadcastStateTransitionRequest stateTransition
                         */

                        /**
                         * Constructs a new BroadcastStateTransitionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BroadcastStateTransitionRequest.
                         * @implements IBroadcastStateTransitionRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest=} [properties] Properties to set
                         */
                        function BroadcastStateTransitionRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BroadcastStateTransitionRequest stateTransition.
                         * @member {Uint8Array} stateTransition
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @instance
                         */
                        BroadcastStateTransitionRequest.prototype.stateTransition = $util.newBuffer([]);

                        /**
                         * Creates a new BroadcastStateTransitionRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest instance
                         */
                        BroadcastStateTransitionRequest.create = function create(properties) {
                            return new BroadcastStateTransitionRequest(properties);
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} message BroadcastStateTransitionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.stateTransition != null && Object.hasOwnProperty.call(message, "stateTransition"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.stateTransition);
                            return writer;
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} message BroadcastStateTransitionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BroadcastStateTransitionRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.stateTransition = reader.bytes();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BroadcastStateTransitionRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BroadcastStateTransitionRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BroadcastStateTransitionRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.stateTransition != null && message.hasOwnProperty("stateTransition"))
                                if (!(message.stateTransition && typeof message.stateTransition.length === "number" || $util.isString(message.stateTransition)))
                                    return "stateTransition: buffer expected";
                            return null;
                        };

                        /**
                         * Creates a BroadcastStateTransitionRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest
                         */
                        BroadcastStateTransitionRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest();
                            if (object.stateTransition != null)
                                if (typeof object.stateTransition === "string")
                                    $util.base64.decode(object.stateTransition, message.stateTransition = $util.newBuffer($util.base64.length(object.stateTransition)), 0);
                                else if (object.stateTransition.length >= 0)
                                    message.stateTransition = object.stateTransition;
                            return message;
                        };

                        /**
                         * Creates a plain object from a BroadcastStateTransitionRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} message BroadcastStateTransitionRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BroadcastStateTransitionRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                if (options.bytes === String)
                                    object.stateTransition = "";
                                else {
                                    object.stateTransition = [];
                                    if (options.bytes !== Array)
                                        object.stateTransition = $util.newBuffer(object.stateTransition);
                                }
                            if (message.stateTransition != null && message.hasOwnProperty("stateTransition"))
                                object.stateTransition = options.bytes === String ? $util.base64.encode(message.stateTransition, 0, message.stateTransition.length) : options.bytes === Array ? Array.prototype.slice.call(message.stateTransition) : message.stateTransition;
                            return object;
                        };

                        /**
                         * Converts this BroadcastStateTransitionRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BroadcastStateTransitionRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BroadcastStateTransitionRequest;
                    })();

                    v0.BroadcastStateTransitionResponse = (function() {

                        /**
                         * Properties of a BroadcastStateTransitionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBroadcastStateTransitionResponse
                         */

                        /**
                         * Constructs a new BroadcastStateTransitionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BroadcastStateTransitionResponse.
                         * @implements IBroadcastStateTransitionResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse=} [properties] Properties to set
                         */
                        function BroadcastStateTransitionResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * Creates a new BroadcastStateTransitionResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse instance
                         */
                        BroadcastStateTransitionResponse.create = function create(properties) {
                            return new BroadcastStateTransitionResponse(properties);
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse} message BroadcastStateTransitionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            return writer;
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse} message BroadcastStateTransitionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BroadcastStateTransitionResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BroadcastStateTransitionResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BroadcastStateTransitionResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BroadcastStateTransitionResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            return null;
                        };

                        /**
                         * Creates a BroadcastStateTransitionResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse
                         */
                        BroadcastStateTransitionResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse)
                                return object;
                            return new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse();
                        };

                        /**
                         * Creates a plain object from a BroadcastStateTransitionResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} message BroadcastStateTransitionResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BroadcastStateTransitionResponse.toObject = function toObject() {
                            return {};
                        };

                        /**
                         * Converts this BroadcastStateTransitionResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BroadcastStateTransitionResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BroadcastStateTransitionResponse;
                    })();

                    v0.GetIdentityRequest = (function() {

                        /**
                         * Properties of a GetIdentityRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityRequest
                         * @property {Uint8Array|null} [id] GetIdentityRequest id
                         * @property {boolean|null} [prove] GetIdentityRequest prove
                         */

                        /**
                         * Constructs a new GetIdentityRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityRequest.
                         * @implements IGetIdentityRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest=} [properties] Properties to set
                         */
                        function GetIdentityRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityRequest id.
                         * @member {Uint8Array} id
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @instance
                         */
                        GetIdentityRequest.prototype.id = $util.newBuffer([]);

                        /**
                         * GetIdentityRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @instance
                         */
                        GetIdentityRequest.prototype.prove = false;

                        /**
                         * Creates a new GetIdentityRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityRequest} GetIdentityRequest instance
                         */
                        GetIdentityRequest.create = function create(properties) {
                            return new GetIdentityRequest(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} message GetIdentityRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.id);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityRequest} message GetIdentityRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityRequest} GetIdentityRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.id = reader.bytes();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityRequest} GetIdentityRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.id != null && message.hasOwnProperty("id"))
                                if (!(message.id && typeof message.id.length === "number" || $util.isString(message.id)))
                                    return "id: buffer expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetIdentityRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityRequest} GetIdentityRequest
                         */
                        GetIdentityRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityRequest();
                            if (object.id != null)
                                if (typeof object.id === "string")
                                    $util.base64.decode(object.id, message.id = $util.newBuffer($util.base64.length(object.id)), 0);
                                else if (object.id.length >= 0)
                                    message.id = object.id;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityRequest} message GetIdentityRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.id = "";
                                else {
                                    object.id = [];
                                    if (options.bytes !== Array)
                                        object.id = $util.newBuffer(object.id);
                                }
                                object.prove = false;
                            }
                            if (message.id != null && message.hasOwnProperty("id"))
                                object.id = options.bytes === String ? $util.base64.encode(message.id, 0, message.id.length) : options.bytes === Array ? Array.prototype.slice.call(message.id) : message.id;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetIdentityRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityRequest;
                    })();

                    v0.GetIdentityResponse = (function() {

                        /**
                         * Properties of a GetIdentityResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityResponse
                         * @property {Uint8Array|null} [identity] GetIdentityResponse identity
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentityResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentityResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentityResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityResponse.
                         * @implements IGetIdentityResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityResponse=} [properties] Properties to set
                         */
                        function GetIdentityResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityResponse identity.
                         * @member {Uint8Array} identity
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @instance
                         */
                        GetIdentityResponse.prototype.identity = $util.newBuffer([]);

                        /**
                         * GetIdentityResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @instance
                         */
                        GetIdentityResponse.prototype.proof = null;

                        /**
                         * GetIdentityResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @instance
                         */
                        GetIdentityResponse.prototype.metadata = null;

                        /**
                         * Creates a new GetIdentityResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityResponse} GetIdentityResponse instance
                         */
                        GetIdentityResponse.create = function create(properties) {
                            return new GetIdentityResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityResponse} message GetIdentityResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.identity != null && Object.hasOwnProperty.call(message, "identity"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.identity);
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityResponse} message GetIdentityResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityResponse} GetIdentityResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.identity = reader.bytes();
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityResponse} GetIdentityResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.identity != null && message.hasOwnProperty("identity"))
                                if (!(message.identity && typeof message.identity.length === "number" || $util.isString(message.identity)))
                                    return "identity: buffer expected";
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                if (error)
                                    return "proof." + error;
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetIdentityResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityResponse} GetIdentityResponse
                         */
                        GetIdentityResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityResponse();
                            if (object.identity != null)
                                if (typeof object.identity === "string")
                                    $util.base64.decode(object.identity, message.identity = $util.newBuffer($util.base64.length(object.identity)), 0);
                                else if (object.identity.length >= 0)
                                    message.identity = object.identity;
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityResponse} message GetIdentityResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.identity = "";
                                else {
                                    object.identity = [];
                                    if (options.bytes !== Array)
                                        object.identity = $util.newBuffer(object.identity);
                                }
                                object.proof = null;
                                object.metadata = null;
                            }
                            if (message.identity != null && message.hasOwnProperty("identity"))
                                object.identity = options.bytes === String ? $util.base64.encode(message.identity, 0, message.identity.length) : options.bytes === Array ? Array.prototype.slice.call(message.identity) : message.identity;
                            if (message.proof != null && message.hasOwnProperty("proof"))
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentityResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityResponse;
                    })();

                    v0.GetIdentityBalanceResponse = (function() {

                        /**
                         * Properties of a GetIdentityBalanceResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityBalanceResponse
                         * @property {google.protobuf.IUInt64Value|null} [balance] GetIdentityBalanceResponse balance
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentityBalanceResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentityBalanceResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentityBalanceResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityBalanceResponse.
                         * @implements IGetIdentityBalanceResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceResponse=} [properties] Properties to set
                         */
                        function GetIdentityBalanceResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityBalanceResponse balance.
                         * @member {google.protobuf.IUInt64Value|null|undefined} balance
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @instance
                         */
                        GetIdentityBalanceResponse.prototype.balance = null;

                        /**
                         * GetIdentityBalanceResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @instance
                         */
                        GetIdentityBalanceResponse.prototype.proof = null;

                        /**
                         * GetIdentityBalanceResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @instance
                         */
                        GetIdentityBalanceResponse.prototype.metadata = null;

                        /**
                         * Creates a new GetIdentityBalanceResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceResponse} GetIdentityBalanceResponse instance
                         */
                        GetIdentityBalanceResponse.create = function create(properties) {
                            return new GetIdentityBalanceResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityBalanceResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityBalanceResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceResponse} message GetIdentityBalanceResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityBalanceResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.balance != null && Object.hasOwnProperty.call(message, "balance"))
                                $root.google.protobuf.UInt64Value.encode(message.balance, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityBalanceResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityBalanceResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceResponse} message GetIdentityBalanceResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityBalanceResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityBalanceResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceResponse} GetIdentityBalanceResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityBalanceResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityBalanceResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.balance = $root.google.protobuf.UInt64Value.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityBalanceResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceResponse} GetIdentityBalanceResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityBalanceResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityBalanceResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityBalanceResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.balance != null && message.hasOwnProperty("balance")) {
                                var error = $root.google.protobuf.UInt64Value.verify(message.balance);
                                if (error)
                                    return "balance." + error;
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                if (error)
                                    return "proof." + error;
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetIdentityBalanceResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceResponse} GetIdentityBalanceResponse
                         */
                        GetIdentityBalanceResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityBalanceResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityBalanceResponse();
                            if (object.balance != null) {
                                if (typeof object.balance !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityBalanceResponse.balance: object expected");
                                message.balance = $root.google.protobuf.UInt64Value.fromObject(object.balance);
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityBalanceResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityBalanceResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityBalanceResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityBalanceResponse} message GetIdentityBalanceResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityBalanceResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.balance = null;
                                object.proof = null;
                                object.metadata = null;
                            }
                            if (message.balance != null && message.hasOwnProperty("balance"))
                                object.balance = $root.google.protobuf.UInt64Value.toObject(message.balance, options);
                            if (message.proof != null && message.hasOwnProperty("proof"))
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentityBalanceResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityBalanceResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityBalanceResponse;
                    })();

                    v0.GetIdentityBalanceAndRevisionResponse = (function() {

                        /**
                         * Properties of a GetIdentityBalanceAndRevisionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityBalanceAndRevisionResponse
                         * @property {google.protobuf.IUInt64Value|null} [balance] GetIdentityBalanceAndRevisionResponse balance
                         * @property {google.protobuf.IUInt64Value|null} [revision] GetIdentityBalanceAndRevisionResponse revision
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentityBalanceAndRevisionResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentityBalanceAndRevisionResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentityBalanceAndRevisionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityBalanceAndRevisionResponse.
                         * @implements IGetIdentityBalanceAndRevisionResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceAndRevisionResponse=} [properties] Properties to set
                         */
                        function GetIdentityBalanceAndRevisionResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityBalanceAndRevisionResponse balance.
                         * @member {google.protobuf.IUInt64Value|null|undefined} balance
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @instance
                         */
                        GetIdentityBalanceAndRevisionResponse.prototype.balance = null;

                        /**
                         * GetIdentityBalanceAndRevisionResponse revision.
                         * @member {google.protobuf.IUInt64Value|null|undefined} revision
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @instance
                         */
                        GetIdentityBalanceAndRevisionResponse.prototype.revision = null;

                        /**
                         * GetIdentityBalanceAndRevisionResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @instance
                         */
                        GetIdentityBalanceAndRevisionResponse.prototype.proof = null;

                        /**
                         * GetIdentityBalanceAndRevisionResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @instance
                         */
                        GetIdentityBalanceAndRevisionResponse.prototype.metadata = null;

                        /**
                         * Creates a new GetIdentityBalanceAndRevisionResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceAndRevisionResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} GetIdentityBalanceAndRevisionResponse instance
                         */
                        GetIdentityBalanceAndRevisionResponse.create = function create(properties) {
                            return new GetIdentityBalanceAndRevisionResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityBalanceAndRevisionResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceAndRevisionResponse} message GetIdentityBalanceAndRevisionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityBalanceAndRevisionResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.balance != null && Object.hasOwnProperty.call(message, "balance"))
                                $root.google.protobuf.UInt64Value.encode(message.balance, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.revision != null && Object.hasOwnProperty.call(message, "revision"))
                                $root.google.protobuf.UInt64Value.encode(message.revision, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 4, wireType 2 =*/34).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityBalanceAndRevisionResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityBalanceAndRevisionResponse} message GetIdentityBalanceAndRevisionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityBalanceAndRevisionResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityBalanceAndRevisionResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} GetIdentityBalanceAndRevisionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityBalanceAndRevisionResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.balance = $root.google.protobuf.UInt64Value.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.revision = $root.google.protobuf.UInt64Value.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 4:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityBalanceAndRevisionResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} GetIdentityBalanceAndRevisionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityBalanceAndRevisionResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityBalanceAndRevisionResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityBalanceAndRevisionResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.balance != null && message.hasOwnProperty("balance")) {
                                var error = $root.google.protobuf.UInt64Value.verify(message.balance);
                                if (error)
                                    return "balance." + error;
                            }
                            if (message.revision != null && message.hasOwnProperty("revision")) {
                                var error = $root.google.protobuf.UInt64Value.verify(message.revision);
                                if (error)
                                    return "revision." + error;
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                if (error)
                                    return "proof." + error;
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetIdentityBalanceAndRevisionResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} GetIdentityBalanceAndRevisionResponse
                         */
                        GetIdentityBalanceAndRevisionResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse();
                            if (object.balance != null) {
                                if (typeof object.balance !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.balance: object expected");
                                message.balance = $root.google.protobuf.UInt64Value.fromObject(object.balance);
                            }
                            if (object.revision != null) {
                                if (typeof object.revision !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.revision: object expected");
                                message.revision = $root.google.protobuf.UInt64Value.fromObject(object.revision);
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityBalanceAndRevisionResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse} message GetIdentityBalanceAndRevisionResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityBalanceAndRevisionResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.balance = null;
                                object.revision = null;
                                object.proof = null;
                                object.metadata = null;
                            }
                            if (message.balance != null && message.hasOwnProperty("balance"))
                                object.balance = $root.google.protobuf.UInt64Value.toObject(message.balance, options);
                            if (message.revision != null && message.hasOwnProperty("revision"))
                                object.revision = $root.google.protobuf.UInt64Value.toObject(message.revision, options);
                            if (message.proof != null && message.hasOwnProperty("proof"))
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentityBalanceAndRevisionResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityBalanceAndRevisionResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityBalanceAndRevisionResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityBalanceAndRevisionResponse;
                    })();

                    v0.KeyRequestType = (function() {

                        /**
                         * Properties of a KeyRequestType.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IKeyRequestType
                         * @property {org.dash.platform.dapi.v0.IAllKeys|null} [allKeys] KeyRequestType allKeys
                         * @property {org.dash.platform.dapi.v0.ISpecificKeys|null} [specificKeys] KeyRequestType specificKeys
                         * @property {org.dash.platform.dapi.v0.ISearchKey|null} [searchKey] KeyRequestType searchKey
                         */

                        /**
                         * Constructs a new KeyRequestType.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a KeyRequestType.
                         * @implements IKeyRequestType
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IKeyRequestType=} [properties] Properties to set
                         */
                        function KeyRequestType(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * KeyRequestType allKeys.
                         * @member {org.dash.platform.dapi.v0.IAllKeys|null|undefined} allKeys
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @instance
                         */
                        KeyRequestType.prototype.allKeys = null;

                        /**
                         * KeyRequestType specificKeys.
                         * @member {org.dash.platform.dapi.v0.ISpecificKeys|null|undefined} specificKeys
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @instance
                         */
                        KeyRequestType.prototype.specificKeys = null;

                        /**
                         * KeyRequestType searchKey.
                         * @member {org.dash.platform.dapi.v0.ISearchKey|null|undefined} searchKey
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @instance
                         */
                        KeyRequestType.prototype.searchKey = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * KeyRequestType request.
                         * @member {"allKeys"|"specificKeys"|"searchKey"|undefined} request
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @instance
                         */
                        Object.defineProperty(KeyRequestType.prototype, "request", {
                            get: $util.oneOfGetter($oneOfFields = ["allKeys", "specificKeys", "searchKey"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new KeyRequestType instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {org.dash.platform.dapi.v0.IKeyRequestType=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.KeyRequestType} KeyRequestType instance
                         */
                        KeyRequestType.create = function create(properties) {
                            return new KeyRequestType(properties);
                        };

                        /**
                         * Encodes the specified KeyRequestType message. Does not implicitly {@link org.dash.platform.dapi.v0.KeyRequestType.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {org.dash.platform.dapi.v0.IKeyRequestType} message KeyRequestType message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        KeyRequestType.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.allKeys != null && Object.hasOwnProperty.call(message, "allKeys"))
                                $root.org.dash.platform.dapi.v0.AllKeys.encode(message.allKeys, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.specificKeys != null && Object.hasOwnProperty.call(message, "specificKeys"))
                                $root.org.dash.platform.dapi.v0.SpecificKeys.encode(message.specificKeys, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.searchKey != null && Object.hasOwnProperty.call(message, "searchKey"))
                                $root.org.dash.platform.dapi.v0.SearchKey.encode(message.searchKey, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified KeyRequestType message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.KeyRequestType.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {org.dash.platform.dapi.v0.IKeyRequestType} message KeyRequestType message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        KeyRequestType.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a KeyRequestType message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.KeyRequestType} KeyRequestType
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        KeyRequestType.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.KeyRequestType();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.allKeys = $root.org.dash.platform.dapi.v0.AllKeys.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.specificKeys = $root.org.dash.platform.dapi.v0.SpecificKeys.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.searchKey = $root.org.dash.platform.dapi.v0.SearchKey.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a KeyRequestType message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.KeyRequestType} KeyRequestType
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        KeyRequestType.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a KeyRequestType message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        KeyRequestType.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.allKeys != null && message.hasOwnProperty("allKeys")) {
                                properties.request = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.AllKeys.verify(message.allKeys);
                                    if (error)
                                        return "allKeys." + error;
                                }
                            }
                            if (message.specificKeys != null && message.hasOwnProperty("specificKeys")) {
                                if (properties.request === 1)
                                    return "request: multiple values";
                                properties.request = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.SpecificKeys.verify(message.specificKeys);
                                    if (error)
                                        return "specificKeys." + error;
                                }
                            }
                            if (message.searchKey != null && message.hasOwnProperty("searchKey")) {
                                if (properties.request === 1)
                                    return "request: multiple values";
                                properties.request = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.SearchKey.verify(message.searchKey);
                                    if (error)
                                        return "searchKey." + error;
                                }
                            }
                            return null;
                        };

                        /**
                         * Creates a KeyRequestType message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.KeyRequestType} KeyRequestType
                         */
                        KeyRequestType.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.KeyRequestType)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.KeyRequestType();
                            if (object.allKeys != null) {
                                if (typeof object.allKeys !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.KeyRequestType.allKeys: object expected");
                                message.allKeys = $root.org.dash.platform.dapi.v0.AllKeys.fromObject(object.allKeys);
                            }
                            if (object.specificKeys != null) {
                                if (typeof object.specificKeys !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.KeyRequestType.specificKeys: object expected");
                                message.specificKeys = $root.org.dash.platform.dapi.v0.SpecificKeys.fromObject(object.specificKeys);
                            }
                            if (object.searchKey != null) {
                                if (typeof object.searchKey !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.KeyRequestType.searchKey: object expected");
                                message.searchKey = $root.org.dash.platform.dapi.v0.SearchKey.fromObject(object.searchKey);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a KeyRequestType message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @static
                         * @param {org.dash.platform.dapi.v0.KeyRequestType} message KeyRequestType
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        KeyRequestType.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (message.allKeys != null && message.hasOwnProperty("allKeys")) {
                                object.allKeys = $root.org.dash.platform.dapi.v0.AllKeys.toObject(message.allKeys, options);
                                if (options.oneofs)
                                    object.request = "allKeys";
                            }
                            if (message.specificKeys != null && message.hasOwnProperty("specificKeys")) {
                                object.specificKeys = $root.org.dash.platform.dapi.v0.SpecificKeys.toObject(message.specificKeys, options);
                                if (options.oneofs)
                                    object.request = "specificKeys";
                            }
                            if (message.searchKey != null && message.hasOwnProperty("searchKey")) {
                                object.searchKey = $root.org.dash.platform.dapi.v0.SearchKey.toObject(message.searchKey, options);
                                if (options.oneofs)
                                    object.request = "searchKey";
                            }
                            return object;
                        };

                        /**
                         * Converts this KeyRequestType to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.KeyRequestType
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        KeyRequestType.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return KeyRequestType;
                    })();

                    v0.AllKeys = (function() {

                        /**
                         * Properties of an AllKeys.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IAllKeys
                         */

                        /**
                         * Constructs a new AllKeys.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents an AllKeys.
                         * @implements IAllKeys
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IAllKeys=} [properties] Properties to set
                         */
                        function AllKeys(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * Creates a new AllKeys instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.IAllKeys=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.AllKeys} AllKeys instance
                         */
                        AllKeys.create = function create(properties) {
                            return new AllKeys(properties);
                        };

                        /**
                         * Encodes the specified AllKeys message. Does not implicitly {@link org.dash.platform.dapi.v0.AllKeys.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.IAllKeys} message AllKeys message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        AllKeys.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            return writer;
                        };

                        /**
                         * Encodes the specified AllKeys message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.AllKeys.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.IAllKeys} message AllKeys message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        AllKeys.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes an AllKeys message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.AllKeys} AllKeys
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        AllKeys.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.AllKeys();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes an AllKeys message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.AllKeys} AllKeys
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        AllKeys.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies an AllKeys message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        AllKeys.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            return null;
                        };

                        /**
                         * Creates an AllKeys message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.AllKeys} AllKeys
                         */
                        AllKeys.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.AllKeys)
                                return object;
                            return new $root.org.dash.platform.dapi.v0.AllKeys();
                        };

                        /**
                         * Creates a plain object from an AllKeys message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.AllKeys} message AllKeys
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        AllKeys.toObject = function toObject() {
                            return {};
                        };

                        /**
                         * Converts this AllKeys to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.AllKeys
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        AllKeys.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return AllKeys;
                    })();

                    v0.SpecificKeys = (function() {

                        /**
                         * Properties of a SpecificKeys.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface ISpecificKeys
                         * @property {Array.<number>|null} [keyIds] SpecificKeys keyIds
                         */

                        /**
                         * Constructs a new SpecificKeys.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a SpecificKeys.
                         * @implements ISpecificKeys
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.ISpecificKeys=} [properties] Properties to set
                         */
                        function SpecificKeys(properties) {
                            this.keyIds = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * SpecificKeys keyIds.
                         * @member {Array.<number>} keyIds
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @instance
                         */
                        SpecificKeys.prototype.keyIds = $util.emptyArray;

                        /**
                         * Creates a new SpecificKeys instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISpecificKeys=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.SpecificKeys} SpecificKeys instance
                         */
                        SpecificKeys.create = function create(properties) {
                            return new SpecificKeys(properties);
                        };

                        /**
                         * Encodes the specified SpecificKeys message. Does not implicitly {@link org.dash.platform.dapi.v0.SpecificKeys.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISpecificKeys} message SpecificKeys message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SpecificKeys.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.keyIds != null && message.keyIds.length) {
                                writer.uint32(/* id 1, wireType 2 =*/10).fork();
                                for (var i = 0; i < message.keyIds.length; ++i)
                                    writer.uint32(message.keyIds[i]);
                                writer.ldelim();
                            }
                            return writer;
                        };

                        /**
                         * Encodes the specified SpecificKeys message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.SpecificKeys.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISpecificKeys} message SpecificKeys message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SpecificKeys.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a SpecificKeys message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.SpecificKeys} SpecificKeys
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SpecificKeys.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.SpecificKeys();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.keyIds && message.keyIds.length))
                                        message.keyIds = [];
                                    if ((tag & 7) === 2) {
                                        var end2 = reader.uint32() + reader.pos;
                                        while (reader.pos < end2)
                                            message.keyIds.push(reader.uint32());
                                    } else
                                        message.keyIds.push(reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a SpecificKeys message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.SpecificKeys} SpecificKeys
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SpecificKeys.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a SpecificKeys message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        SpecificKeys.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.keyIds != null && message.hasOwnProperty("keyIds")) {
                                if (!Array.isArray(message.keyIds))
                                    return "keyIds: array expected";
                                for (var i = 0; i < message.keyIds.length; ++i)
                                    if (!$util.isInteger(message.keyIds[i]))
                                        return "keyIds: integer[] expected";
                            }
                            return null;
                        };

                        /**
                         * Creates a SpecificKeys message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.SpecificKeys} SpecificKeys
                         */
                        SpecificKeys.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.SpecificKeys)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.SpecificKeys();
                            if (object.keyIds) {
                                if (!Array.isArray(object.keyIds))
                                    throw TypeError(".org.dash.platform.dapi.v0.SpecificKeys.keyIds: array expected");
                                message.keyIds = [];
                                for (var i = 0; i < object.keyIds.length; ++i)
                                    message.keyIds[i] = object.keyIds[i] >>> 0;
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a SpecificKeys message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @static
                         * @param {org.dash.platform.dapi.v0.SpecificKeys} message SpecificKeys
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        SpecificKeys.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.keyIds = [];
                            if (message.keyIds && message.keyIds.length) {
                                object.keyIds = [];
                                for (var j = 0; j < message.keyIds.length; ++j)
                                    object.keyIds[j] = message.keyIds[j];
                            }
                            return object;
                        };

                        /**
                         * Converts this SpecificKeys to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.SpecificKeys
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        SpecificKeys.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return SpecificKeys;
                    })();

                    v0.SearchKey = (function() {

                        /**
                         * Properties of a SearchKey.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface ISearchKey
                         * @property {Object.<string,org.dash.platform.dapi.v0.ISecurityLevelMap>|null} [purposeMap] SearchKey purposeMap
                         */

                        /**
                         * Constructs a new SearchKey.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a SearchKey.
                         * @implements ISearchKey
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.ISearchKey=} [properties] Properties to set
                         */
                        function SearchKey(properties) {
                            this.purposeMap = {};
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * SearchKey purposeMap.
                         * @member {Object.<string,org.dash.platform.dapi.v0.ISecurityLevelMap>} purposeMap
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @instance
                         */
                        SearchKey.prototype.purposeMap = $util.emptyObject;

                        /**
                         * Creates a new SearchKey instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISearchKey=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.SearchKey} SearchKey instance
                         */
                        SearchKey.create = function create(properties) {
                            return new SearchKey(properties);
                        };

                        /**
                         * Encodes the specified SearchKey message. Does not implicitly {@link org.dash.platform.dapi.v0.SearchKey.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISearchKey} message SearchKey message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SearchKey.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.purposeMap != null && Object.hasOwnProperty.call(message, "purposeMap"))
                                for (var keys = Object.keys(message.purposeMap), i = 0; i < keys.length; ++i) {
                                    writer.uint32(/* id 1, wireType 2 =*/10).fork().uint32(/* id 1, wireType 0 =*/8).uint32(keys[i]);
                                    $root.org.dash.platform.dapi.v0.SecurityLevelMap.encode(message.purposeMap[keys[i]], writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim().ldelim();
                                }
                            return writer;
                        };

                        /**
                         * Encodes the specified SearchKey message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.SearchKey.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISearchKey} message SearchKey message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SearchKey.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a SearchKey message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.SearchKey} SearchKey
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SearchKey.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.SearchKey(), key, value;
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (message.purposeMap === $util.emptyObject)
                                        message.purposeMap = {};
                                    var end2 = reader.uint32() + reader.pos;
                                    key = 0;
                                    value = null;
                                    while (reader.pos < end2) {
                                        var tag2 = reader.uint32();
                                        switch (tag2 >>> 3) {
                                        case 1:
                                            key = reader.uint32();
                                            break;
                                        case 2:
                                            value = $root.org.dash.platform.dapi.v0.SecurityLevelMap.decode(reader, reader.uint32());
                                            break;
                                        default:
                                            reader.skipType(tag2 & 7);
                                            break;
                                        }
                                    }
                                    message.purposeMap[key] = value;
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a SearchKey message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.SearchKey} SearchKey
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SearchKey.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a SearchKey message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        SearchKey.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.purposeMap != null && message.hasOwnProperty("purposeMap")) {
                                if (!$util.isObject(message.purposeMap))
                                    return "purposeMap: object expected";
                                var key = Object.keys(message.purposeMap);
                                for (var i = 0; i < key.length; ++i) {
                                    if (!$util.key32Re.test(key[i]))
                                        return "purposeMap: integer key{k:uint32} expected";
                                    {
                                        var error = $root.org.dash.platform.dapi.v0.SecurityLevelMap.verify(message.purposeMap[key[i]]);
                                        if (error)
                                            return "purposeMap." + error;
                                    }
                                }
                            }
                            return null;
                        };

                        /**
                         * Creates a SearchKey message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.SearchKey} SearchKey
                         */
                        SearchKey.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.SearchKey)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.SearchKey();
                            if (object.purposeMap) {
                                if (typeof object.purposeMap !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.SearchKey.purposeMap: object expected");
                                message.purposeMap = {};
                                for (var keys = Object.keys(object.purposeMap), i = 0; i < keys.length; ++i) {
                                    if (typeof object.purposeMap[keys[i]] !== "object")
                                        throw TypeError(".org.dash.platform.dapi.v0.SearchKey.purposeMap: object expected");
                                    message.purposeMap[keys[i]] = $root.org.dash.platform.dapi.v0.SecurityLevelMap.fromObject(object.purposeMap[keys[i]]);
                                }
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a SearchKey message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @static
                         * @param {org.dash.platform.dapi.v0.SearchKey} message SearchKey
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        SearchKey.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.objects || options.defaults)
                                object.purposeMap = {};
                            var keys2;
                            if (message.purposeMap && (keys2 = Object.keys(message.purposeMap)).length) {
                                object.purposeMap = {};
                                for (var j = 0; j < keys2.length; ++j)
                                    object.purposeMap[keys2[j]] = $root.org.dash.platform.dapi.v0.SecurityLevelMap.toObject(message.purposeMap[keys2[j]], options);
                            }
                            return object;
                        };

                        /**
                         * Converts this SearchKey to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.SearchKey
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        SearchKey.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return SearchKey;
                    })();

                    v0.SecurityLevelMap = (function() {

                        /**
                         * Properties of a SecurityLevelMap.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface ISecurityLevelMap
                         * @property {Object.<string,org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType>|null} [securityLevelMap] SecurityLevelMap securityLevelMap
                         */

                        /**
                         * Constructs a new SecurityLevelMap.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a SecurityLevelMap.
                         * @implements ISecurityLevelMap
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.ISecurityLevelMap=} [properties] Properties to set
                         */
                        function SecurityLevelMap(properties) {
                            this.securityLevelMap = {};
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * SecurityLevelMap securityLevelMap.
                         * @member {Object.<string,org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType>} securityLevelMap
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @instance
                         */
                        SecurityLevelMap.prototype.securityLevelMap = $util.emptyObject;

                        /**
                         * Creates a new SecurityLevelMap instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISecurityLevelMap=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.SecurityLevelMap} SecurityLevelMap instance
                         */
                        SecurityLevelMap.create = function create(properties) {
                            return new SecurityLevelMap(properties);
                        };

                        /**
                         * Encodes the specified SecurityLevelMap message. Does not implicitly {@link org.dash.platform.dapi.v0.SecurityLevelMap.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISecurityLevelMap} message SecurityLevelMap message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SecurityLevelMap.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.securityLevelMap != null && Object.hasOwnProperty.call(message, "securityLevelMap"))
                                for (var keys = Object.keys(message.securityLevelMap), i = 0; i < keys.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).fork().uint32(/* id 1, wireType 0 =*/8).uint32(keys[i]).uint32(/* id 2, wireType 0 =*/16).int32(message.securityLevelMap[keys[i]]).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified SecurityLevelMap message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.SecurityLevelMap.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISecurityLevelMap} message SecurityLevelMap message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SecurityLevelMap.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a SecurityLevelMap message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.SecurityLevelMap} SecurityLevelMap
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SecurityLevelMap.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.SecurityLevelMap(), key, value;
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (message.securityLevelMap === $util.emptyObject)
                                        message.securityLevelMap = {};
                                    var end2 = reader.uint32() + reader.pos;
                                    key = 0;
                                    value = 0;
                                    while (reader.pos < end2) {
                                        var tag2 = reader.uint32();
                                        switch (tag2 >>> 3) {
                                        case 1:
                                            key = reader.uint32();
                                            break;
                                        case 2:
                                            value = reader.int32();
                                            break;
                                        default:
                                            reader.skipType(tag2 & 7);
                                            break;
                                        }
                                    }
                                    message.securityLevelMap[key] = value;
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a SecurityLevelMap message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.SecurityLevelMap} SecurityLevelMap
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SecurityLevelMap.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a SecurityLevelMap message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        SecurityLevelMap.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.securityLevelMap != null && message.hasOwnProperty("securityLevelMap")) {
                                if (!$util.isObject(message.securityLevelMap))
                                    return "securityLevelMap: object expected";
                                var key = Object.keys(message.securityLevelMap);
                                for (var i = 0; i < key.length; ++i) {
                                    if (!$util.key32Re.test(key[i]))
                                        return "securityLevelMap: integer key{k:uint32} expected";
                                    switch (message.securityLevelMap[key[i]]) {
                                    default:
                                        return "securityLevelMap: enum value{k:uint32} expected";
                                    case 0:
                                    case 1:
                                        break;
                                    }
                                }
                            }
                            return null;
                        };

                        /**
                         * Creates a SecurityLevelMap message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.SecurityLevelMap} SecurityLevelMap
                         */
                        SecurityLevelMap.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.SecurityLevelMap)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.SecurityLevelMap();
                            if (object.securityLevelMap) {
                                if (typeof object.securityLevelMap !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.SecurityLevelMap.securityLevelMap: object expected");
                                message.securityLevelMap = {};
                                for (var keys = Object.keys(object.securityLevelMap), i = 0; i < keys.length; ++i)
                                    switch (object.securityLevelMap[keys[i]]) {
                                    case "CURRENT_KEY_OF_KIND_REQUEST":
                                    case 0:
                                        message.securityLevelMap[keys[i]] = 0;
                                        break;
                                    case "ALL_KEYS_OF_KIND_REQUEST":
                                    case 1:
                                        message.securityLevelMap[keys[i]] = 1;
                                        break;
                                    }
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a SecurityLevelMap message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @static
                         * @param {org.dash.platform.dapi.v0.SecurityLevelMap} message SecurityLevelMap
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        SecurityLevelMap.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.objects || options.defaults)
                                object.securityLevelMap = {};
                            var keys2;
                            if (message.securityLevelMap && (keys2 = Object.keys(message.securityLevelMap)).length) {
                                object.securityLevelMap = {};
                                for (var j = 0; j < keys2.length; ++j)
                                    object.securityLevelMap[keys2[j]] = options.enums === String ? $root.org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType[message.securityLevelMap[keys2[j]]] : message.securityLevelMap[keys2[j]];
                            }
                            return object;
                        };

                        /**
                         * Converts this SecurityLevelMap to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.SecurityLevelMap
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        SecurityLevelMap.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        /**
                         * KeyKindRequestType enum.
                         * @name org.dash.platform.dapi.v0.SecurityLevelMap.KeyKindRequestType
                         * @enum {number}
                         * @property {number} CURRENT_KEY_OF_KIND_REQUEST=0 CURRENT_KEY_OF_KIND_REQUEST value
                         * @property {number} ALL_KEYS_OF_KIND_REQUEST=1 ALL_KEYS_OF_KIND_REQUEST value
                         */
                        SecurityLevelMap.KeyKindRequestType = (function() {
                            var valuesById = {}, values = Object.create(valuesById);
                            values[valuesById[0] = "CURRENT_KEY_OF_KIND_REQUEST"] = 0;
                            values[valuesById[1] = "ALL_KEYS_OF_KIND_REQUEST"] = 1;
                            return values;
                        })();

                        return SecurityLevelMap;
                    })();

                    v0.GetIdentityKeysRequest = (function() {

                        /**
                         * Properties of a GetIdentityKeysRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityKeysRequest
                         * @property {Uint8Array|null} [identityId] GetIdentityKeysRequest identityId
                         * @property {org.dash.platform.dapi.v0.IKeyRequestType|null} [requestType] GetIdentityKeysRequest requestType
                         * @property {google.protobuf.IUInt32Value|null} [limit] GetIdentityKeysRequest limit
                         * @property {google.protobuf.IUInt32Value|null} [offset] GetIdentityKeysRequest offset
                         * @property {boolean|null} [prove] GetIdentityKeysRequest prove
                         */

                        /**
                         * Constructs a new GetIdentityKeysRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityKeysRequest.
                         * @implements IGetIdentityKeysRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysRequest=} [properties] Properties to set
                         */
                        function GetIdentityKeysRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityKeysRequest identityId.
                         * @member {Uint8Array} identityId
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @instance
                         */
                        GetIdentityKeysRequest.prototype.identityId = $util.newBuffer([]);

                        /**
                         * GetIdentityKeysRequest requestType.
                         * @member {org.dash.platform.dapi.v0.IKeyRequestType|null|undefined} requestType
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @instance
                         */
                        GetIdentityKeysRequest.prototype.requestType = null;

                        /**
                         * GetIdentityKeysRequest limit.
                         * @member {google.protobuf.IUInt32Value|null|undefined} limit
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @instance
                         */
                        GetIdentityKeysRequest.prototype.limit = null;

                        /**
                         * GetIdentityKeysRequest offset.
                         * @member {google.protobuf.IUInt32Value|null|undefined} offset
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @instance
                         */
                        GetIdentityKeysRequest.prototype.offset = null;

                        /**
                         * GetIdentityKeysRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @instance
                         */
                        GetIdentityKeysRequest.prototype.prove = false;

                        /**
                         * Creates a new GetIdentityKeysRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysRequest} GetIdentityKeysRequest instance
                         */
                        GetIdentityKeysRequest.create = function create(properties) {
                            return new GetIdentityKeysRequest(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityKeysRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityKeysRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysRequest} message GetIdentityKeysRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityKeysRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.identityId != null && Object.hasOwnProperty.call(message, "identityId"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.identityId);
                            if (message.requestType != null && Object.hasOwnProperty.call(message, "requestType"))
                                $root.org.dash.platform.dapi.v0.KeyRequestType.encode(message.requestType, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.limit != null && Object.hasOwnProperty.call(message, "limit"))
                                $root.google.protobuf.UInt32Value.encode(message.limit, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            if (message.offset != null && Object.hasOwnProperty.call(message, "offset"))
                                $root.google.protobuf.UInt32Value.encode(message.offset, writer.uint32(/* id 4, wireType 2 =*/34).fork()).ldelim();
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 5, wireType 0 =*/40).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityKeysRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityKeysRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysRequest} message GetIdentityKeysRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityKeysRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityKeysRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysRequest} GetIdentityKeysRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityKeysRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityKeysRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.identityId = reader.bytes();
                                    break;
                                case 2:
                                    message.requestType = $root.org.dash.platform.dapi.v0.KeyRequestType.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.limit = $root.google.protobuf.UInt32Value.decode(reader, reader.uint32());
                                    break;
                                case 4:
                                    message.offset = $root.google.protobuf.UInt32Value.decode(reader, reader.uint32());
                                    break;
                                case 5:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityKeysRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysRequest} GetIdentityKeysRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityKeysRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityKeysRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityKeysRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.identityId != null && message.hasOwnProperty("identityId"))
                                if (!(message.identityId && typeof message.identityId.length === "number" || $util.isString(message.identityId)))
                                    return "identityId: buffer expected";
                            if (message.requestType != null && message.hasOwnProperty("requestType")) {
                                var error = $root.org.dash.platform.dapi.v0.KeyRequestType.verify(message.requestType);
                                if (error)
                                    return "requestType." + error;
                            }
                            if (message.limit != null && message.hasOwnProperty("limit")) {
                                var error = $root.google.protobuf.UInt32Value.verify(message.limit);
                                if (error)
                                    return "limit." + error;
                            }
                            if (message.offset != null && message.hasOwnProperty("offset")) {
                                var error = $root.google.protobuf.UInt32Value.verify(message.offset);
                                if (error)
                                    return "offset." + error;
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetIdentityKeysRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysRequest} GetIdentityKeysRequest
                         */
                        GetIdentityKeysRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityKeysRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityKeysRequest();
                            if (object.identityId != null)
                                if (typeof object.identityId === "string")
                                    $util.base64.decode(object.identityId, message.identityId = $util.newBuffer($util.base64.length(object.identityId)), 0);
                                else if (object.identityId.length >= 0)
                                    message.identityId = object.identityId;
                            if (object.requestType != null) {
                                if (typeof object.requestType !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityKeysRequest.requestType: object expected");
                                message.requestType = $root.org.dash.platform.dapi.v0.KeyRequestType.fromObject(object.requestType);
                            }
                            if (object.limit != null) {
                                if (typeof object.limit !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityKeysRequest.limit: object expected");
                                message.limit = $root.google.protobuf.UInt32Value.fromObject(object.limit);
                            }
                            if (object.offset != null) {
                                if (typeof object.offset !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityKeysRequest.offset: object expected");
                                message.offset = $root.google.protobuf.UInt32Value.fromObject(object.offset);
                            }
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityKeysRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityKeysRequest} message GetIdentityKeysRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityKeysRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.identityId = "";
                                else {
                                    object.identityId = [];
                                    if (options.bytes !== Array)
                                        object.identityId = $util.newBuffer(object.identityId);
                                }
                                object.requestType = null;
                                object.limit = null;
                                object.offset = null;
                                object.prove = false;
                            }
                            if (message.identityId != null && message.hasOwnProperty("identityId"))
                                object.identityId = options.bytes === String ? $util.base64.encode(message.identityId, 0, message.identityId.length) : options.bytes === Array ? Array.prototype.slice.call(message.identityId) : message.identityId;
                            if (message.requestType != null && message.hasOwnProperty("requestType"))
                                object.requestType = $root.org.dash.platform.dapi.v0.KeyRequestType.toObject(message.requestType, options);
                            if (message.limit != null && message.hasOwnProperty("limit"))
                                object.limit = $root.google.protobuf.UInt32Value.toObject(message.limit, options);
                            if (message.offset != null && message.hasOwnProperty("offset"))
                                object.offset = $root.google.protobuf.UInt32Value.toObject(message.offset, options);
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetIdentityKeysRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityKeysRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityKeysRequest;
                    })();

                    v0.GetIdentityKeysResponse = (function() {

                        /**
                         * Properties of a GetIdentityKeysResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityKeysResponse
                         * @property {org.dash.platform.dapi.v0.GetIdentityKeysResponse.IKeys|null} [keys] GetIdentityKeysResponse keys
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentityKeysResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentityKeysResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentityKeysResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityKeysResponse.
                         * @implements IGetIdentityKeysResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysResponse=} [properties] Properties to set
                         */
                        function GetIdentityKeysResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityKeysResponse keys.
                         * @member {org.dash.platform.dapi.v0.GetIdentityKeysResponse.IKeys|null|undefined} keys
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @instance
                         */
                        GetIdentityKeysResponse.prototype.keys = null;

                        /**
                         * GetIdentityKeysResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @instance
                         */
                        GetIdentityKeysResponse.prototype.proof = null;

                        /**
                         * GetIdentityKeysResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @instance
                         */
                        GetIdentityKeysResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * GetIdentityKeysResponse result.
                         * @member {"keys"|"proof"|undefined} result
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @instance
                         */
                        Object.defineProperty(GetIdentityKeysResponse.prototype, "result", {
                            get: $util.oneOfGetter($oneOfFields = ["keys", "proof"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new GetIdentityKeysResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse} GetIdentityKeysResponse instance
                         */
                        GetIdentityKeysResponse.create = function create(properties) {
                            return new GetIdentityKeysResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityKeysResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityKeysResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysResponse} message GetIdentityKeysResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityKeysResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.keys != null && Object.hasOwnProperty.call(message, "keys"))
                                $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.encode(message.keys, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityKeysResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityKeysResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityKeysResponse} message GetIdentityKeysResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityKeysResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityKeysResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse} GetIdentityKeysResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityKeysResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.keys = $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityKeysResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse} GetIdentityKeysResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityKeysResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityKeysResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityKeysResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.keys != null && message.hasOwnProperty("keys")) {
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.verify(message.keys);
                                    if (error)
                                        return "keys." + error;
                                }
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                if (properties.result === 1)
                                    return "result: multiple values";
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                    if (error)
                                        return "proof." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetIdentityKeysResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse} GetIdentityKeysResponse
                         */
                        GetIdentityKeysResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse();
                            if (object.keys != null) {
                                if (typeof object.keys !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityKeysResponse.keys: object expected");
                                message.keys = $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.fromObject(object.keys);
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityKeysResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityKeysResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityKeysResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityKeysResponse} message GetIdentityKeysResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityKeysResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.keys != null && message.hasOwnProperty("keys")) {
                                object.keys = $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.toObject(message.keys, options);
                                if (options.oneofs)
                                    object.result = "keys";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                                if (options.oneofs)
                                    object.result = "proof";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentityKeysResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityKeysResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        GetIdentityKeysResponse.Keys = (function() {

                            /**
                             * Properties of a Keys.
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                             * @interface IKeys
                             * @property {Array.<Uint8Array>|null} [keysBytes] Keys keysBytes
                             */

                            /**
                             * Constructs a new Keys.
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse
                             * @classdesc Represents a Keys.
                             * @implements IKeys
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetIdentityKeysResponse.IKeys=} [properties] Properties to set
                             */
                            function Keys(properties) {
                                this.keysBytes = [];
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * Keys keysBytes.
                             * @member {Array.<Uint8Array>} keysBytes
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @instance
                             */
                            Keys.prototype.keysBytes = $util.emptyArray;

                            /**
                             * Creates a new Keys instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentityKeysResponse.IKeys=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} Keys instance
                             */
                            Keys.create = function create(properties) {
                                return new Keys(properties);
                            };

                            /**
                             * Encodes the specified Keys message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentityKeysResponse.IKeys} message Keys message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Keys.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.keysBytes != null && message.keysBytes.length)
                                    for (var i = 0; i < message.keysBytes.length; ++i)
                                        writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.keysBytes[i]);
                                return writer;
                            };

                            /**
                             * Encodes the specified Keys message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentityKeysResponse.IKeys} message Keys message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Keys.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a Keys message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} Keys
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Keys.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        if (!(message.keysBytes && message.keysBytes.length))
                                            message.keysBytes = [];
                                        message.keysBytes.push(reader.bytes());
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a Keys message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} Keys
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Keys.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a Keys message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            Keys.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.keysBytes != null && message.hasOwnProperty("keysBytes")) {
                                    if (!Array.isArray(message.keysBytes))
                                        return "keysBytes: array expected";
                                    for (var i = 0; i < message.keysBytes.length; ++i)
                                        if (!(message.keysBytes[i] && typeof message.keysBytes[i].length === "number" || $util.isString(message.keysBytes[i])))
                                            return "keysBytes: buffer[] expected";
                                }
                                return null;
                            };

                            /**
                             * Creates a Keys message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} Keys
                             */
                            Keys.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys();
                                if (object.keysBytes) {
                                    if (!Array.isArray(object.keysBytes))
                                        throw TypeError(".org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys.keysBytes: array expected");
                                    message.keysBytes = [];
                                    for (var i = 0; i < object.keysBytes.length; ++i)
                                        if (typeof object.keysBytes[i] === "string")
                                            $util.base64.decode(object.keysBytes[i], message.keysBytes[i] = $util.newBuffer($util.base64.length(object.keysBytes[i])), 0);
                                        else if (object.keysBytes[i].length >= 0)
                                            message.keysBytes[i] = object.keysBytes[i];
                                }
                                return message;
                            };

                            /**
                             * Creates a plain object from a Keys message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys} message Keys
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            Keys.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.arrays || options.defaults)
                                    object.keysBytes = [];
                                if (message.keysBytes && message.keysBytes.length) {
                                    object.keysBytes = [];
                                    for (var j = 0; j < message.keysBytes.length; ++j)
                                        object.keysBytes[j] = options.bytes === String ? $util.base64.encode(message.keysBytes[j], 0, message.keysBytes[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.keysBytes[j]) : message.keysBytes[j];
                                }
                                return object;
                            };

                            /**
                             * Converts this Keys to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetIdentityKeysResponse.Keys
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            Keys.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return Keys;
                        })();

                        return GetIdentityKeysResponse;
                    })();

                    v0.GetIdentitiesKeysRequest = (function() {

                        /**
                         * Properties of a GetIdentitiesKeysRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentitiesKeysRequest
                         * @property {Array.<Uint8Array>|null} [identityIds] GetIdentitiesKeysRequest identityIds
                         * @property {org.dash.platform.dapi.v0.IKeyRequestType|null} [requestType] GetIdentitiesKeysRequest requestType
                         * @property {google.protobuf.IUInt32Value|null} [limit] GetIdentitiesKeysRequest limit
                         * @property {google.protobuf.IUInt32Value|null} [offset] GetIdentitiesKeysRequest offset
                         * @property {boolean|null} [prove] GetIdentitiesKeysRequest prove
                         */

                        /**
                         * Constructs a new GetIdentitiesKeysRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentitiesKeysRequest.
                         * @implements IGetIdentitiesKeysRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysRequest=} [properties] Properties to set
                         */
                        function GetIdentitiesKeysRequest(properties) {
                            this.identityIds = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentitiesKeysRequest identityIds.
                         * @member {Array.<Uint8Array>} identityIds
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @instance
                         */
                        GetIdentitiesKeysRequest.prototype.identityIds = $util.emptyArray;

                        /**
                         * GetIdentitiesKeysRequest requestType.
                         * @member {org.dash.platform.dapi.v0.IKeyRequestType|null|undefined} requestType
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @instance
                         */
                        GetIdentitiesKeysRequest.prototype.requestType = null;

                        /**
                         * GetIdentitiesKeysRequest limit.
                         * @member {google.protobuf.IUInt32Value|null|undefined} limit
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @instance
                         */
                        GetIdentitiesKeysRequest.prototype.limit = null;

                        /**
                         * GetIdentitiesKeysRequest offset.
                         * @member {google.protobuf.IUInt32Value|null|undefined} offset
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @instance
                         */
                        GetIdentitiesKeysRequest.prototype.offset = null;

                        /**
                         * GetIdentitiesKeysRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @instance
                         */
                        GetIdentitiesKeysRequest.prototype.prove = false;

                        /**
                         * Creates a new GetIdentitiesKeysRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} GetIdentitiesKeysRequest instance
                         */
                        GetIdentitiesKeysRequest.create = function create(properties) {
                            return new GetIdentitiesKeysRequest(properties);
                        };

                        /**
                         * Encodes the specified GetIdentitiesKeysRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysRequest} message GetIdentitiesKeysRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesKeysRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.identityIds != null && message.identityIds.length)
                                for (var i = 0; i < message.identityIds.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.identityIds[i]);
                            if (message.requestType != null && Object.hasOwnProperty.call(message, "requestType"))
                                $root.org.dash.platform.dapi.v0.KeyRequestType.encode(message.requestType, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.limit != null && Object.hasOwnProperty.call(message, "limit"))
                                $root.google.protobuf.UInt32Value.encode(message.limit, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            if (message.offset != null && Object.hasOwnProperty.call(message, "offset"))
                                $root.google.protobuf.UInt32Value.encode(message.offset, writer.uint32(/* id 4, wireType 2 =*/34).fork()).ldelim();
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 5, wireType 0 =*/40).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentitiesKeysRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysRequest} message GetIdentitiesKeysRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesKeysRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentitiesKeysRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} GetIdentitiesKeysRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesKeysRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.identityIds && message.identityIds.length))
                                        message.identityIds = [];
                                    message.identityIds.push(reader.bytes());
                                    break;
                                case 2:
                                    message.requestType = $root.org.dash.platform.dapi.v0.KeyRequestType.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.limit = $root.google.protobuf.UInt32Value.decode(reader, reader.uint32());
                                    break;
                                case 4:
                                    message.offset = $root.google.protobuf.UInt32Value.decode(reader, reader.uint32());
                                    break;
                                case 5:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentitiesKeysRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} GetIdentitiesKeysRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesKeysRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentitiesKeysRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentitiesKeysRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.identityIds != null && message.hasOwnProperty("identityIds")) {
                                if (!Array.isArray(message.identityIds))
                                    return "identityIds: array expected";
                                for (var i = 0; i < message.identityIds.length; ++i)
                                    if (!(message.identityIds[i] && typeof message.identityIds[i].length === "number" || $util.isString(message.identityIds[i])))
                                        return "identityIds: buffer[] expected";
                            }
                            if (message.requestType != null && message.hasOwnProperty("requestType")) {
                                var error = $root.org.dash.platform.dapi.v0.KeyRequestType.verify(message.requestType);
                                if (error)
                                    return "requestType." + error;
                            }
                            if (message.limit != null && message.hasOwnProperty("limit")) {
                                var error = $root.google.protobuf.UInt32Value.verify(message.limit);
                                if (error)
                                    return "limit." + error;
                            }
                            if (message.offset != null && message.hasOwnProperty("offset")) {
                                var error = $root.google.protobuf.UInt32Value.verify(message.offset);
                                if (error)
                                    return "offset." + error;
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetIdentitiesKeysRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} GetIdentitiesKeysRequest
                         */
                        GetIdentitiesKeysRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest();
                            if (object.identityIds) {
                                if (!Array.isArray(object.identityIds))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.identityIds: array expected");
                                message.identityIds = [];
                                for (var i = 0; i < object.identityIds.length; ++i)
                                    if (typeof object.identityIds[i] === "string")
                                        $util.base64.decode(object.identityIds[i], message.identityIds[i] = $util.newBuffer($util.base64.length(object.identityIds[i])), 0);
                                    else if (object.identityIds[i].length >= 0)
                                        message.identityIds[i] = object.identityIds[i];
                            }
                            if (object.requestType != null) {
                                if (typeof object.requestType !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.requestType: object expected");
                                message.requestType = $root.org.dash.platform.dapi.v0.KeyRequestType.fromObject(object.requestType);
                            }
                            if (object.limit != null) {
                                if (typeof object.limit !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.limit: object expected");
                                message.limit = $root.google.protobuf.UInt32Value.fromObject(object.limit);
                            }
                            if (object.offset != null) {
                                if (typeof object.offset !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.offset: object expected");
                                message.offset = $root.google.protobuf.UInt32Value.fromObject(object.offset);
                            }
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentitiesKeysRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest} message GetIdentitiesKeysRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentitiesKeysRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.identityIds = [];
                            if (options.defaults) {
                                object.requestType = null;
                                object.limit = null;
                                object.offset = null;
                                object.prove = false;
                            }
                            if (message.identityIds && message.identityIds.length) {
                                object.identityIds = [];
                                for (var j = 0; j < message.identityIds.length; ++j)
                                    object.identityIds[j] = options.bytes === String ? $util.base64.encode(message.identityIds[j], 0, message.identityIds[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.identityIds[j]) : message.identityIds[j];
                            }
                            if (message.requestType != null && message.hasOwnProperty("requestType"))
                                object.requestType = $root.org.dash.platform.dapi.v0.KeyRequestType.toObject(message.requestType, options);
                            if (message.limit != null && message.hasOwnProperty("limit"))
                                object.limit = $root.google.protobuf.UInt32Value.toObject(message.limit, options);
                            if (message.offset != null && message.hasOwnProperty("offset"))
                                object.offset = $root.google.protobuf.UInt32Value.toObject(message.offset, options);
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetIdentitiesKeysRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentitiesKeysRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        GetIdentitiesKeysRequest.SecurityLevelMap = (function() {

                            /**
                             * Properties of a SecurityLevelMap.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                             * @interface ISecurityLevelMap
                             * @property {Object.<string,org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType>|null} [securityLevelMap] SecurityLevelMap securityLevelMap
                             */

                            /**
                             * Constructs a new SecurityLevelMap.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest
                             * @classdesc Represents a SecurityLevelMap.
                             * @implements ISecurityLevelMap
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.ISecurityLevelMap=} [properties] Properties to set
                             */
                            function SecurityLevelMap(properties) {
                                this.securityLevelMap = {};
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * SecurityLevelMap securityLevelMap.
                             * @member {Object.<string,org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType>} securityLevelMap
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @instance
                             */
                            SecurityLevelMap.prototype.securityLevelMap = $util.emptyObject;

                            /**
                             * Creates a new SecurityLevelMap instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.ISecurityLevelMap=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} SecurityLevelMap instance
                             */
                            SecurityLevelMap.create = function create(properties) {
                                return new SecurityLevelMap(properties);
                            };

                            /**
                             * Encodes the specified SecurityLevelMap message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.ISecurityLevelMap} message SecurityLevelMap message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            SecurityLevelMap.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.securityLevelMap != null && Object.hasOwnProperty.call(message, "securityLevelMap"))
                                    for (var keys = Object.keys(message.securityLevelMap), i = 0; i < keys.length; ++i)
                                        writer.uint32(/* id 1, wireType 2 =*/10).fork().uint32(/* id 1, wireType 0 =*/8).uint32(keys[i]).uint32(/* id 2, wireType 0 =*/16).int32(message.securityLevelMap[keys[i]]).ldelim();
                                return writer;
                            };

                            /**
                             * Encodes the specified SecurityLevelMap message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.ISecurityLevelMap} message SecurityLevelMap message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            SecurityLevelMap.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a SecurityLevelMap message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} SecurityLevelMap
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            SecurityLevelMap.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap(), key, value;
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        if (message.securityLevelMap === $util.emptyObject)
                                            message.securityLevelMap = {};
                                        var end2 = reader.uint32() + reader.pos;
                                        key = 0;
                                        value = 0;
                                        while (reader.pos < end2) {
                                            var tag2 = reader.uint32();
                                            switch (tag2 >>> 3) {
                                            case 1:
                                                key = reader.uint32();
                                                break;
                                            case 2:
                                                value = reader.int32();
                                                break;
                                            default:
                                                reader.skipType(tag2 & 7);
                                                break;
                                            }
                                        }
                                        message.securityLevelMap[key] = value;
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a SecurityLevelMap message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} SecurityLevelMap
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            SecurityLevelMap.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a SecurityLevelMap message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            SecurityLevelMap.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.securityLevelMap != null && message.hasOwnProperty("securityLevelMap")) {
                                    if (!$util.isObject(message.securityLevelMap))
                                        return "securityLevelMap: object expected";
                                    var key = Object.keys(message.securityLevelMap);
                                    for (var i = 0; i < key.length; ++i) {
                                        if (!$util.key32Re.test(key[i]))
                                            return "securityLevelMap: integer key{k:uint32} expected";
                                        switch (message.securityLevelMap[key[i]]) {
                                        default:
                                            return "securityLevelMap: enum value{k:uint32} expected";
                                        case 0:
                                            break;
                                        }
                                    }
                                }
                                return null;
                            };

                            /**
                             * Creates a SecurityLevelMap message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} SecurityLevelMap
                             */
                            SecurityLevelMap.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap();
                                if (object.securityLevelMap) {
                                    if (typeof object.securityLevelMap !== "object")
                                        throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.securityLevelMap: object expected");
                                    message.securityLevelMap = {};
                                    for (var keys = Object.keys(object.securityLevelMap), i = 0; i < keys.length; ++i)
                                        switch (object.securityLevelMap[keys[i]]) {
                                        case "CURRENT_KEY_OF_KIND_REQUEST":
                                        case 0:
                                            message.securityLevelMap[keys[i]] = 0;
                                            break;
                                        }
                                }
                                return message;
                            };

                            /**
                             * Creates a plain object from a SecurityLevelMap message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap} message SecurityLevelMap
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            SecurityLevelMap.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.objects || options.defaults)
                                    object.securityLevelMap = {};
                                var keys2;
                                if (message.securityLevelMap && (keys2 = Object.keys(message.securityLevelMap)).length) {
                                    object.securityLevelMap = {};
                                    for (var j = 0; j < keys2.length; ++j)
                                        object.securityLevelMap[keys2[j]] = options.enums === String ? $root.org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType[message.securityLevelMap[keys2[j]]] : message.securityLevelMap[keys2[j]];
                                }
                                return object;
                            };

                            /**
                             * Converts this SecurityLevelMap to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            SecurityLevelMap.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            /**
                             * KeyKindRequestType enum.
                             * @name org.dash.platform.dapi.v0.GetIdentitiesKeysRequest.SecurityLevelMap.KeyKindRequestType
                             * @enum {number}
                             * @property {number} CURRENT_KEY_OF_KIND_REQUEST=0 CURRENT_KEY_OF_KIND_REQUEST value
                             */
                            SecurityLevelMap.KeyKindRequestType = (function() {
                                var valuesById = {}, values = Object.create(valuesById);
                                values[valuesById[0] = "CURRENT_KEY_OF_KIND_REQUEST"] = 0;
                                return values;
                            })();

                            return SecurityLevelMap;
                        })();

                        return GetIdentitiesKeysRequest;
                    })();

                    v0.GetIdentitiesKeysResponse = (function() {

                        /**
                         * Properties of a GetIdentitiesKeysResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentitiesKeysResponse
                         * @property {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntries|null} [publicKeys] GetIdentitiesKeysResponse publicKeys
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentitiesKeysResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentitiesKeysResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentitiesKeysResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentitiesKeysResponse.
                         * @implements IGetIdentitiesKeysResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysResponse=} [properties] Properties to set
                         */
                        function GetIdentitiesKeysResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentitiesKeysResponse publicKeys.
                         * @member {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntries|null|undefined} publicKeys
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @instance
                         */
                        GetIdentitiesKeysResponse.prototype.publicKeys = null;

                        /**
                         * GetIdentitiesKeysResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @instance
                         */
                        GetIdentitiesKeysResponse.prototype.proof = null;

                        /**
                         * GetIdentitiesKeysResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @instance
                         */
                        GetIdentitiesKeysResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * GetIdentitiesKeysResponse result.
                         * @member {"publicKeys"|"proof"|undefined} result
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @instance
                         */
                        Object.defineProperty(GetIdentitiesKeysResponse.prototype, "result", {
                            get: $util.oneOfGetter($oneOfFields = ["publicKeys", "proof"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new GetIdentitiesKeysResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} GetIdentitiesKeysResponse instance
                         */
                        GetIdentitiesKeysResponse.create = function create(properties) {
                            return new GetIdentitiesKeysResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentitiesKeysResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysResponse} message GetIdentitiesKeysResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesKeysResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.publicKeys != null && Object.hasOwnProperty.call(message, "publicKeys"))
                                $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.encode(message.publicKeys, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentitiesKeysResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesKeysResponse} message GetIdentitiesKeysResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesKeysResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentitiesKeysResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} GetIdentitiesKeysResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesKeysResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.publicKeys = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentitiesKeysResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} GetIdentitiesKeysResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesKeysResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentitiesKeysResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentitiesKeysResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.publicKeys != null && message.hasOwnProperty("publicKeys")) {
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.verify(message.publicKeys);
                                    if (error)
                                        return "publicKeys." + error;
                                }
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                if (properties.result === 1)
                                    return "result: multiple values";
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                    if (error)
                                        return "proof." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetIdentitiesKeysResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} GetIdentitiesKeysResponse
                         */
                        GetIdentitiesKeysResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse();
                            if (object.publicKeys != null) {
                                if (typeof object.publicKeys !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.publicKeys: object expected");
                                message.publicKeys = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.fromObject(object.publicKeys);
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentitiesKeysResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse} message GetIdentitiesKeysResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentitiesKeysResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.publicKeys != null && message.hasOwnProperty("publicKeys")) {
                                object.publicKeys = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.toObject(message.publicKeys, options);
                                if (options.oneofs)
                                    object.result = "publicKeys";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                                if (options.oneofs)
                                    object.result = "proof";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentitiesKeysResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentitiesKeysResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        GetIdentitiesKeysResponse.PublicKey = (function() {

                            /**
                             * Properties of a PublicKey.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                             * @interface IPublicKey
                             * @property {Uint8Array|null} [value] PublicKey value
                             */

                            /**
                             * Constructs a new PublicKey.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                             * @classdesc Represents a PublicKey.
                             * @implements IPublicKey
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKey=} [properties] Properties to set
                             */
                            function PublicKey(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * PublicKey value.
                             * @member {Uint8Array} value
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @instance
                             */
                            PublicKey.prototype.value = $util.newBuffer([]);

                            /**
                             * Creates a new PublicKey instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKey=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} PublicKey instance
                             */
                            PublicKey.create = function create(properties) {
                                return new PublicKey(properties);
                            };

                            /**
                             * Encodes the specified PublicKey message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKey} message PublicKey message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            PublicKey.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.value);
                                return writer;
                            };

                            /**
                             * Encodes the specified PublicKey message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKey} message PublicKey message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            PublicKey.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a PublicKey message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} PublicKey
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            PublicKey.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.value = reader.bytes();
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a PublicKey message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} PublicKey
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            PublicKey.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a PublicKey message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            PublicKey.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.value != null && message.hasOwnProperty("value"))
                                    if (!(message.value && typeof message.value.length === "number" || $util.isString(message.value)))
                                        return "value: buffer expected";
                                return null;
                            };

                            /**
                             * Creates a PublicKey message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} PublicKey
                             */
                            PublicKey.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey();
                                if (object.value != null)
                                    if (typeof object.value === "string")
                                        $util.base64.decode(object.value, message.value = $util.newBuffer($util.base64.length(object.value)), 0);
                                    else if (object.value.length >= 0)
                                        message.value = object.value;
                                return message;
                            };

                            /**
                             * Creates a plain object from a PublicKey message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey} message PublicKey
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            PublicKey.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults)
                                    if (options.bytes === String)
                                        object.value = "";
                                    else {
                                        object.value = [];
                                        if (options.bytes !== Array)
                                            object.value = $util.newBuffer(object.value);
                                    }
                                if (message.value != null && message.hasOwnProperty("value"))
                                    object.value = options.bytes === String ? $util.base64.encode(message.value, 0, message.value.length) : options.bytes === Array ? Array.prototype.slice.call(message.value) : message.value;
                                return object;
                            };

                            /**
                             * Converts this PublicKey to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            PublicKey.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return PublicKey;
                        })();

                        GetIdentitiesKeysResponse.PublicKeyEntry = (function() {

                            /**
                             * Properties of a PublicKeyEntry.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                             * @interface IPublicKeyEntry
                             * @property {Uint8Array|null} [key] PublicKeyEntry key
                             * @property {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKey|null} [value] PublicKeyEntry value
                             */

                            /**
                             * Constructs a new PublicKeyEntry.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                             * @classdesc Represents a PublicKeyEntry.
                             * @implements IPublicKeyEntry
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntry=} [properties] Properties to set
                             */
                            function PublicKeyEntry(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * PublicKeyEntry key.
                             * @member {Uint8Array} key
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @instance
                             */
                            PublicKeyEntry.prototype.key = $util.newBuffer([]);

                            /**
                             * PublicKeyEntry value.
                             * @member {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKey|null|undefined} value
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @instance
                             */
                            PublicKeyEntry.prototype.value = null;

                            /**
                             * Creates a new PublicKeyEntry instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntry=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} PublicKeyEntry instance
                             */
                            PublicKeyEntry.create = function create(properties) {
                                return new PublicKeyEntry(properties);
                            };

                            /**
                             * Encodes the specified PublicKeyEntry message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntry} message PublicKeyEntry message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            PublicKeyEntry.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.key != null && Object.hasOwnProperty.call(message, "key"))
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.key);
                                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                                    $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.encode(message.value, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                                return writer;
                            };

                            /**
                             * Encodes the specified PublicKeyEntry message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntry} message PublicKeyEntry message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            PublicKeyEntry.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a PublicKeyEntry message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} PublicKeyEntry
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            PublicKeyEntry.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.key = reader.bytes();
                                        break;
                                    case 2:
                                        message.value = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.decode(reader, reader.uint32());
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a PublicKeyEntry message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} PublicKeyEntry
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            PublicKeyEntry.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a PublicKeyEntry message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            PublicKeyEntry.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.key != null && message.hasOwnProperty("key"))
                                    if (!(message.key && typeof message.key.length === "number" || $util.isString(message.key)))
                                        return "key: buffer expected";
                                if (message.value != null && message.hasOwnProperty("value")) {
                                    var error = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.verify(message.value);
                                    if (error)
                                        return "value." + error;
                                }
                                return null;
                            };

                            /**
                             * Creates a PublicKeyEntry message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} PublicKeyEntry
                             */
                            PublicKeyEntry.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry();
                                if (object.key != null)
                                    if (typeof object.key === "string")
                                        $util.base64.decode(object.key, message.key = $util.newBuffer($util.base64.length(object.key)), 0);
                                    else if (object.key.length >= 0)
                                        message.key = object.key;
                                if (object.value != null) {
                                    if (typeof object.value !== "object")
                                        throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.value: object expected");
                                    message.value = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.fromObject(object.value);
                                }
                                return message;
                            };

                            /**
                             * Creates a plain object from a PublicKeyEntry message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry} message PublicKeyEntry
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            PublicKeyEntry.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    if (options.bytes === String)
                                        object.key = "";
                                    else {
                                        object.key = [];
                                        if (options.bytes !== Array)
                                            object.key = $util.newBuffer(object.key);
                                    }
                                    object.value = null;
                                }
                                if (message.key != null && message.hasOwnProperty("key"))
                                    object.key = options.bytes === String ? $util.base64.encode(message.key, 0, message.key.length) : options.bytes === Array ? Array.prototype.slice.call(message.key) : message.key;
                                if (message.value != null && message.hasOwnProperty("value"))
                                    object.value = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKey.toObject(message.value, options);
                                return object;
                            };

                            /**
                             * Converts this PublicKeyEntry to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            PublicKeyEntry.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return PublicKeyEntry;
                        })();

                        GetIdentitiesKeysResponse.PublicKeyEntries = (function() {

                            /**
                             * Properties of a PublicKeyEntries.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                             * @interface IPublicKeyEntries
                             * @property {Array.<org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntry>|null} [publicKeyEntries] PublicKeyEntries publicKeyEntries
                             */

                            /**
                             * Constructs a new PublicKeyEntries.
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse
                             * @classdesc Represents a PublicKeyEntries.
                             * @implements IPublicKeyEntries
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntries=} [properties] Properties to set
                             */
                            function PublicKeyEntries(properties) {
                                this.publicKeyEntries = [];
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * PublicKeyEntries publicKeyEntries.
                             * @member {Array.<org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntry>} publicKeyEntries
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @instance
                             */
                            PublicKeyEntries.prototype.publicKeyEntries = $util.emptyArray;

                            /**
                             * Creates a new PublicKeyEntries instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntries=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} PublicKeyEntries instance
                             */
                            PublicKeyEntries.create = function create(properties) {
                                return new PublicKeyEntries(properties);
                            };

                            /**
                             * Encodes the specified PublicKeyEntries message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntries} message PublicKeyEntries message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            PublicKeyEntries.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.publicKeyEntries != null && message.publicKeyEntries.length)
                                    for (var i = 0; i < message.publicKeyEntries.length; ++i)
                                        $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.encode(message.publicKeyEntries[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                                return writer;
                            };

                            /**
                             * Encodes the specified PublicKeyEntries message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.IPublicKeyEntries} message PublicKeyEntries message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            PublicKeyEntries.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a PublicKeyEntries message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} PublicKeyEntries
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            PublicKeyEntries.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        if (!(message.publicKeyEntries && message.publicKeyEntries.length))
                                            message.publicKeyEntries = [];
                                        message.publicKeyEntries.push($root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.decode(reader, reader.uint32()));
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a PublicKeyEntries message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} PublicKeyEntries
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            PublicKeyEntries.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a PublicKeyEntries message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            PublicKeyEntries.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.publicKeyEntries != null && message.hasOwnProperty("publicKeyEntries")) {
                                    if (!Array.isArray(message.publicKeyEntries))
                                        return "publicKeyEntries: array expected";
                                    for (var i = 0; i < message.publicKeyEntries.length; ++i) {
                                        var error = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.verify(message.publicKeyEntries[i]);
                                        if (error)
                                            return "publicKeyEntries." + error;
                                    }
                                }
                                return null;
                            };

                            /**
                             * Creates a PublicKeyEntries message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} PublicKeyEntries
                             */
                            PublicKeyEntries.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries();
                                if (object.publicKeyEntries) {
                                    if (!Array.isArray(object.publicKeyEntries))
                                        throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.publicKeyEntries: array expected");
                                    message.publicKeyEntries = [];
                                    for (var i = 0; i < object.publicKeyEntries.length; ++i) {
                                        if (typeof object.publicKeyEntries[i] !== "object")
                                            throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries.publicKeyEntries: object expected");
                                        message.publicKeyEntries[i] = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.fromObject(object.publicKeyEntries[i]);
                                    }
                                }
                                return message;
                            };

                            /**
                             * Creates a plain object from a PublicKeyEntries message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries} message PublicKeyEntries
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            PublicKeyEntries.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.arrays || options.defaults)
                                    object.publicKeyEntries = [];
                                if (message.publicKeyEntries && message.publicKeyEntries.length) {
                                    object.publicKeyEntries = [];
                                    for (var j = 0; j < message.publicKeyEntries.length; ++j)
                                        object.publicKeyEntries[j] = $root.org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntry.toObject(message.publicKeyEntries[j], options);
                                }
                                return object;
                            };

                            /**
                             * Converts this PublicKeyEntries to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetIdentitiesKeysResponse.PublicKeyEntries
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            PublicKeyEntries.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return PublicKeyEntries;
                        })();

                        return GetIdentitiesKeysResponse;
                    })();

                    v0.GetDataContractRequest = (function() {

                        /**
                         * Properties of a GetDataContractRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetDataContractRequest
                         * @property {Uint8Array|null} [id] GetDataContractRequest id
                         * @property {boolean|null} [prove] GetDataContractRequest prove
                         */

                        /**
                         * Constructs a new GetDataContractRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetDataContractRequest.
                         * @implements IGetDataContractRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetDataContractRequest=} [properties] Properties to set
                         */
                        function GetDataContractRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetDataContractRequest id.
                         * @member {Uint8Array} id
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @instance
                         */
                        GetDataContractRequest.prototype.id = $util.newBuffer([]);

                        /**
                         * GetDataContractRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @instance
                         */
                        GetDataContractRequest.prototype.prove = false;

                        /**
                         * Creates a new GetDataContractRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetDataContractRequest} GetDataContractRequest instance
                         */
                        GetDataContractRequest.create = function create(properties) {
                            return new GetDataContractRequest(properties);
                        };

                        /**
                         * Encodes the specified GetDataContractRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractRequest} message GetDataContractRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.id);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetDataContractRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractRequest} message GetDataContractRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetDataContractRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetDataContractRequest} GetDataContractRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDataContractRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.id = reader.bytes();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetDataContractRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetDataContractRequest} GetDataContractRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetDataContractRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetDataContractRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.id != null && message.hasOwnProperty("id"))
                                if (!(message.id && typeof message.id.length === "number" || $util.isString(message.id)))
                                    return "id: buffer expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetDataContractRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetDataContractRequest} GetDataContractRequest
                         */
                        GetDataContractRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetDataContractRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetDataContractRequest();
                            if (object.id != null)
                                if (typeof object.id === "string")
                                    $util.base64.decode(object.id, message.id = $util.newBuffer($util.base64.length(object.id)), 0);
                                else if (object.id.length >= 0)
                                    message.id = object.id;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetDataContractRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetDataContractRequest} message GetDataContractRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetDataContractRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.id = "";
                                else {
                                    object.id = [];
                                    if (options.bytes !== Array)
                                        object.id = $util.newBuffer(object.id);
                                }
                                object.prove = false;
                            }
                            if (message.id != null && message.hasOwnProperty("id"))
                                object.id = options.bytes === String ? $util.base64.encode(message.id, 0, message.id.length) : options.bytes === Array ? Array.prototype.slice.call(message.id) : message.id;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetDataContractRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetDataContractRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetDataContractRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetDataContractRequest;
                    })();

                    v0.GetDataContractResponse = (function() {

                        /**
                         * Properties of a GetDataContractResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetDataContractResponse
                         * @property {Uint8Array|null} [dataContract] GetDataContractResponse dataContract
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetDataContractResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetDataContractResponse metadata
                         */

                        /**
                         * Constructs a new GetDataContractResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetDataContractResponse.
                         * @implements IGetDataContractResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetDataContractResponse=} [properties] Properties to set
                         */
                        function GetDataContractResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetDataContractResponse dataContract.
                         * @member {Uint8Array} dataContract
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @instance
                         */
                        GetDataContractResponse.prototype.dataContract = $util.newBuffer([]);

                        /**
                         * GetDataContractResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @instance
                         */
                        GetDataContractResponse.prototype.proof = null;

                        /**
                         * GetDataContractResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @instance
                         */
                        GetDataContractResponse.prototype.metadata = null;

                        /**
                         * Creates a new GetDataContractResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetDataContractResponse} GetDataContractResponse instance
                         */
                        GetDataContractResponse.create = function create(properties) {
                            return new GetDataContractResponse(properties);
                        };

                        /**
                         * Encodes the specified GetDataContractResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractResponse} message GetDataContractResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.dataContract != null && Object.hasOwnProperty.call(message, "dataContract"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.dataContract);
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetDataContractResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractResponse} message GetDataContractResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetDataContractResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetDataContractResponse} GetDataContractResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDataContractResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.dataContract = reader.bytes();
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetDataContractResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetDataContractResponse} GetDataContractResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetDataContractResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetDataContractResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.dataContract != null && message.hasOwnProperty("dataContract"))
                                if (!(message.dataContract && typeof message.dataContract.length === "number" || $util.isString(message.dataContract)))
                                    return "dataContract: buffer expected";
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                if (error)
                                    return "proof." + error;
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetDataContractResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetDataContractResponse} GetDataContractResponse
                         */
                        GetDataContractResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetDataContractResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetDataContractResponse();
                            if (object.dataContract != null)
                                if (typeof object.dataContract === "string")
                                    $util.base64.decode(object.dataContract, message.dataContract = $util.newBuffer($util.base64.length(object.dataContract)), 0);
                                else if (object.dataContract.length >= 0)
                                    message.dataContract = object.dataContract;
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDataContractResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDataContractResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetDataContractResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetDataContractResponse} message GetDataContractResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetDataContractResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.dataContract = "";
                                else {
                                    object.dataContract = [];
                                    if (options.bytes !== Array)
                                        object.dataContract = $util.newBuffer(object.dataContract);
                                }
                                object.proof = null;
                                object.metadata = null;
                            }
                            if (message.dataContract != null && message.hasOwnProperty("dataContract"))
                                object.dataContract = options.bytes === String ? $util.base64.encode(message.dataContract, 0, message.dataContract.length) : options.bytes === Array ? Array.prototype.slice.call(message.dataContract) : message.dataContract;
                            if (message.proof != null && message.hasOwnProperty("proof"))
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetDataContractResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetDataContractResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetDataContractResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetDataContractResponse;
                    })();

                    v0.GetDataContractsRequest = (function() {

                        /**
                         * Properties of a GetDataContractsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetDataContractsRequest
                         * @property {Array.<Uint8Array>|null} [ids] GetDataContractsRequest ids
                         * @property {boolean|null} [prove] GetDataContractsRequest prove
                         */

                        /**
                         * Constructs a new GetDataContractsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetDataContractsRequest.
                         * @implements IGetDataContractsRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsRequest=} [properties] Properties to set
                         */
                        function GetDataContractsRequest(properties) {
                            this.ids = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetDataContractsRequest ids.
                         * @member {Array.<Uint8Array>} ids
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @instance
                         */
                        GetDataContractsRequest.prototype.ids = $util.emptyArray;

                        /**
                         * GetDataContractsRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @instance
                         */
                        GetDataContractsRequest.prototype.prove = false;

                        /**
                         * Creates a new GetDataContractsRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsRequest} GetDataContractsRequest instance
                         */
                        GetDataContractsRequest.create = function create(properties) {
                            return new GetDataContractsRequest(properties);
                        };

                        /**
                         * Encodes the specified GetDataContractsRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsRequest} message GetDataContractsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractsRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.ids != null && message.ids.length)
                                for (var i = 0; i < message.ids.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.ids[i]);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetDataContractsRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsRequest} message GetDataContractsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractsRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetDataContractsRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsRequest} GetDataContractsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractsRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDataContractsRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.ids && message.ids.length))
                                        message.ids = [];
                                    message.ids.push(reader.bytes());
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetDataContractsRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsRequest} GetDataContractsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractsRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetDataContractsRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetDataContractsRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.ids != null && message.hasOwnProperty("ids")) {
                                if (!Array.isArray(message.ids))
                                    return "ids: array expected";
                                for (var i = 0; i < message.ids.length; ++i)
                                    if (!(message.ids[i] && typeof message.ids[i].length === "number" || $util.isString(message.ids[i])))
                                        return "ids: buffer[] expected";
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetDataContractsRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsRequest} GetDataContractsRequest
                         */
                        GetDataContractsRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetDataContractsRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetDataContractsRequest();
                            if (object.ids) {
                                if (!Array.isArray(object.ids))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDataContractsRequest.ids: array expected");
                                message.ids = [];
                                for (var i = 0; i < object.ids.length; ++i)
                                    if (typeof object.ids[i] === "string")
                                        $util.base64.decode(object.ids[i], message.ids[i] = $util.newBuffer($util.base64.length(object.ids[i])), 0);
                                    else if (object.ids[i].length >= 0)
                                        message.ids[i] = object.ids[i];
                            }
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetDataContractsRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetDataContractsRequest} message GetDataContractsRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetDataContractsRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.ids = [];
                            if (options.defaults)
                                object.prove = false;
                            if (message.ids && message.ids.length) {
                                object.ids = [];
                                for (var j = 0; j < message.ids.length; ++j)
                                    object.ids[j] = options.bytes === String ? $util.base64.encode(message.ids[j], 0, message.ids[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.ids[j]) : message.ids[j];
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetDataContractsRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetDataContractsRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetDataContractsRequest;
                    })();

                    v0.GetDataContractsResponse = (function() {

                        /**
                         * Properties of a GetDataContractsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetDataContractsResponse
                         * @property {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContracts|null} [dataContracts] GetDataContractsResponse dataContracts
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetDataContractsResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetDataContractsResponse metadata
                         */

                        /**
                         * Constructs a new GetDataContractsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetDataContractsResponse.
                         * @implements IGetDataContractsResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsResponse=} [properties] Properties to set
                         */
                        function GetDataContractsResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetDataContractsResponse dataContracts.
                         * @member {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContracts|null|undefined} dataContracts
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @instance
                         */
                        GetDataContractsResponse.prototype.dataContracts = null;

                        /**
                         * GetDataContractsResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @instance
                         */
                        GetDataContractsResponse.prototype.proof = null;

                        /**
                         * GetDataContractsResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @instance
                         */
                        GetDataContractsResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * GetDataContractsResponse result.
                         * @member {"dataContracts"|"proof"|undefined} result
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @instance
                         */
                        Object.defineProperty(GetDataContractsResponse.prototype, "result", {
                            get: $util.oneOfGetter($oneOfFields = ["dataContracts", "proof"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new GetDataContractsResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse} GetDataContractsResponse instance
                         */
                        GetDataContractsResponse.create = function create(properties) {
                            return new GetDataContractsResponse(properties);
                        };

                        /**
                         * Encodes the specified GetDataContractsResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsResponse} message GetDataContractsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractsResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.dataContracts != null && Object.hasOwnProperty.call(message, "dataContracts"))
                                $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.encode(message.dataContracts, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetDataContractsResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDataContractsResponse} message GetDataContractsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDataContractsResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetDataContractsResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse} GetDataContractsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractsResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.dataContracts = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetDataContractsResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse} GetDataContractsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDataContractsResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetDataContractsResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetDataContractsResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.dataContracts != null && message.hasOwnProperty("dataContracts")) {
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.verify(message.dataContracts);
                                    if (error)
                                        return "dataContracts." + error;
                                }
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                if (properties.result === 1)
                                    return "result: multiple values";
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                    if (error)
                                        return "proof." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetDataContractsResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse} GetDataContractsResponse
                         */
                        GetDataContractsResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetDataContractsResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse();
                            if (object.dataContracts != null) {
                                if (typeof object.dataContracts !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDataContractsResponse.dataContracts: object expected");
                                message.dataContracts = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.fromObject(object.dataContracts);
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDataContractsResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDataContractsResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetDataContractsResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetDataContractsResponse} message GetDataContractsResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetDataContractsResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.dataContracts != null && message.hasOwnProperty("dataContracts")) {
                                object.dataContracts = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.toObject(message.dataContracts, options);
                                if (options.oneofs)
                                    object.result = "dataContracts";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                                if (options.oneofs)
                                    object.result = "proof";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetDataContractsResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetDataContractsResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        GetDataContractsResponse.DataContractValue = (function() {

                            /**
                             * Properties of a DataContractValue.
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                             * @interface IDataContractValue
                             * @property {Uint8Array|null} [value] DataContractValue value
                             */

                            /**
                             * Constructs a new DataContractValue.
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                             * @classdesc Represents a DataContractValue.
                             * @implements IDataContractValue
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractValue=} [properties] Properties to set
                             */
                            function DataContractValue(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * DataContractValue value.
                             * @member {Uint8Array} value
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @instance
                             */
                            DataContractValue.prototype.value = $util.newBuffer([]);

                            /**
                             * Creates a new DataContractValue instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractValue=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} DataContractValue instance
                             */
                            DataContractValue.create = function create(properties) {
                                return new DataContractValue(properties);
                            };

                            /**
                             * Encodes the specified DataContractValue message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractValue} message DataContractValue message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            DataContractValue.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.value);
                                return writer;
                            };

                            /**
                             * Encodes the specified DataContractValue message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractValue} message DataContractValue message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            DataContractValue.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a DataContractValue message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} DataContractValue
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            DataContractValue.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.value = reader.bytes();
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a DataContractValue message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} DataContractValue
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            DataContractValue.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a DataContractValue message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            DataContractValue.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.value != null && message.hasOwnProperty("value"))
                                    if (!(message.value && typeof message.value.length === "number" || $util.isString(message.value)))
                                        return "value: buffer expected";
                                return null;
                            };

                            /**
                             * Creates a DataContractValue message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} DataContractValue
                             */
                            DataContractValue.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue();
                                if (object.value != null)
                                    if (typeof object.value === "string")
                                        $util.base64.decode(object.value, message.value = $util.newBuffer($util.base64.length(object.value)), 0);
                                    else if (object.value.length >= 0)
                                        message.value = object.value;
                                return message;
                            };

                            /**
                             * Creates a plain object from a DataContractValue message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue} message DataContractValue
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            DataContractValue.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults)
                                    if (options.bytes === String)
                                        object.value = "";
                                    else {
                                        object.value = [];
                                        if (options.bytes !== Array)
                                            object.value = $util.newBuffer(object.value);
                                    }
                                if (message.value != null && message.hasOwnProperty("value"))
                                    object.value = options.bytes === String ? $util.base64.encode(message.value, 0, message.value.length) : options.bytes === Array ? Array.prototype.slice.call(message.value) : message.value;
                                return object;
                            };

                            /**
                             * Converts this DataContractValue to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            DataContractValue.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return DataContractValue;
                        })();

                        GetDataContractsResponse.DataContractEntry = (function() {

                            /**
                             * Properties of a DataContractEntry.
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                             * @interface IDataContractEntry
                             * @property {Uint8Array|null} [key] DataContractEntry key
                             * @property {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractValue|null} [value] DataContractEntry value
                             */

                            /**
                             * Constructs a new DataContractEntry.
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                             * @classdesc Represents a DataContractEntry.
                             * @implements IDataContractEntry
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractEntry=} [properties] Properties to set
                             */
                            function DataContractEntry(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * DataContractEntry key.
                             * @member {Uint8Array} key
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @instance
                             */
                            DataContractEntry.prototype.key = $util.newBuffer([]);

                            /**
                             * DataContractEntry value.
                             * @member {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractValue|null|undefined} value
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @instance
                             */
                            DataContractEntry.prototype.value = null;

                            /**
                             * Creates a new DataContractEntry instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractEntry=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} DataContractEntry instance
                             */
                            DataContractEntry.create = function create(properties) {
                                return new DataContractEntry(properties);
                            };

                            /**
                             * Encodes the specified DataContractEntry message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractEntry} message DataContractEntry message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            DataContractEntry.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.key != null && Object.hasOwnProperty.call(message, "key"))
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.key);
                                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                                    $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.encode(message.value, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                                return writer;
                            };

                            /**
                             * Encodes the specified DataContractEntry message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractEntry} message DataContractEntry message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            DataContractEntry.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a DataContractEntry message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} DataContractEntry
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            DataContractEntry.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.key = reader.bytes();
                                        break;
                                    case 2:
                                        message.value = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.decode(reader, reader.uint32());
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a DataContractEntry message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} DataContractEntry
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            DataContractEntry.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a DataContractEntry message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            DataContractEntry.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.key != null && message.hasOwnProperty("key"))
                                    if (!(message.key && typeof message.key.length === "number" || $util.isString(message.key)))
                                        return "key: buffer expected";
                                if (message.value != null && message.hasOwnProperty("value")) {
                                    var error = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.verify(message.value);
                                    if (error)
                                        return "value." + error;
                                }
                                return null;
                            };

                            /**
                             * Creates a DataContractEntry message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} DataContractEntry
                             */
                            DataContractEntry.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry();
                                if (object.key != null)
                                    if (typeof object.key === "string")
                                        $util.base64.decode(object.key, message.key = $util.newBuffer($util.base64.length(object.key)), 0);
                                    else if (object.key.length >= 0)
                                        message.key = object.key;
                                if (object.value != null) {
                                    if (typeof object.value !== "object")
                                        throw TypeError(".org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.value: object expected");
                                    message.value = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.fromObject(object.value);
                                }
                                return message;
                            };

                            /**
                             * Creates a plain object from a DataContractEntry message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry} message DataContractEntry
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            DataContractEntry.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    if (options.bytes === String)
                                        object.key = "";
                                    else {
                                        object.key = [];
                                        if (options.bytes !== Array)
                                            object.key = $util.newBuffer(object.key);
                                    }
                                    object.value = null;
                                }
                                if (message.key != null && message.hasOwnProperty("key"))
                                    object.key = options.bytes === String ? $util.base64.encode(message.key, 0, message.key.length) : options.bytes === Array ? Array.prototype.slice.call(message.key) : message.key;
                                if (message.value != null && message.hasOwnProperty("value"))
                                    object.value = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractValue.toObject(message.value, options);
                                return object;
                            };

                            /**
                             * Converts this DataContractEntry to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            DataContractEntry.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return DataContractEntry;
                        })();

                        GetDataContractsResponse.DataContracts = (function() {

                            /**
                             * Properties of a DataContracts.
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                             * @interface IDataContracts
                             * @property {Array.<org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractEntry>|null} [dataContractEntries] DataContracts dataContractEntries
                             */

                            /**
                             * Constructs a new DataContracts.
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse
                             * @classdesc Represents a DataContracts.
                             * @implements IDataContracts
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContracts=} [properties] Properties to set
                             */
                            function DataContracts(properties) {
                                this.dataContractEntries = [];
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * DataContracts dataContractEntries.
                             * @member {Array.<org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContractEntry>} dataContractEntries
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @instance
                             */
                            DataContracts.prototype.dataContractEntries = $util.emptyArray;

                            /**
                             * Creates a new DataContracts instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContracts=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} DataContracts instance
                             */
                            DataContracts.create = function create(properties) {
                                return new DataContracts(properties);
                            };

                            /**
                             * Encodes the specified DataContracts message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContracts} message DataContracts message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            DataContracts.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.dataContractEntries != null && message.dataContractEntries.length)
                                    for (var i = 0; i < message.dataContractEntries.length; ++i)
                                        $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.encode(message.dataContractEntries[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                                return writer;
                            };

                            /**
                             * Encodes the specified DataContracts message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.IDataContracts} message DataContracts message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            DataContracts.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a DataContracts message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} DataContracts
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            DataContracts.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        if (!(message.dataContractEntries && message.dataContractEntries.length))
                                            message.dataContractEntries = [];
                                        message.dataContractEntries.push($root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.decode(reader, reader.uint32()));
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a DataContracts message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} DataContracts
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            DataContracts.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a DataContracts message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            DataContracts.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.dataContractEntries != null && message.hasOwnProperty("dataContractEntries")) {
                                    if (!Array.isArray(message.dataContractEntries))
                                        return "dataContractEntries: array expected";
                                    for (var i = 0; i < message.dataContractEntries.length; ++i) {
                                        var error = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.verify(message.dataContractEntries[i]);
                                        if (error)
                                            return "dataContractEntries." + error;
                                    }
                                }
                                return null;
                            };

                            /**
                             * Creates a DataContracts message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} DataContracts
                             */
                            DataContracts.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts();
                                if (object.dataContractEntries) {
                                    if (!Array.isArray(object.dataContractEntries))
                                        throw TypeError(".org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.dataContractEntries: array expected");
                                    message.dataContractEntries = [];
                                    for (var i = 0; i < object.dataContractEntries.length; ++i) {
                                        if (typeof object.dataContractEntries[i] !== "object")
                                            throw TypeError(".org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts.dataContractEntries: object expected");
                                        message.dataContractEntries[i] = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.fromObject(object.dataContractEntries[i]);
                                    }
                                }
                                return message;
                            };

                            /**
                             * Creates a plain object from a DataContracts message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts} message DataContracts
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            DataContracts.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.arrays || options.defaults)
                                    object.dataContractEntries = [];
                                if (message.dataContractEntries && message.dataContractEntries.length) {
                                    object.dataContractEntries = [];
                                    for (var j = 0; j < message.dataContractEntries.length; ++j)
                                        object.dataContractEntries[j] = $root.org.dash.platform.dapi.v0.GetDataContractsResponse.DataContractEntry.toObject(message.dataContractEntries[j], options);
                                }
                                return object;
                            };

                            /**
                             * Converts this DataContracts to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetDataContractsResponse.DataContracts
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            DataContracts.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return DataContracts;
                        })();

                        return GetDataContractsResponse;
                    })();

                    v0.GetDocumentsRequest = (function() {

                        /**
                         * Properties of a GetDocumentsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetDocumentsRequest
                         * @property {Uint8Array|null} [dataContractId] GetDocumentsRequest dataContractId
                         * @property {string|null} [documentType] GetDocumentsRequest documentType
                         * @property {Uint8Array|null} [where] GetDocumentsRequest where
                         * @property {Uint8Array|null} [orderBy] GetDocumentsRequest orderBy
                         * @property {number|null} [limit] GetDocumentsRequest limit
                         * @property {Uint8Array|null} [startAfter] GetDocumentsRequest startAfter
                         * @property {Uint8Array|null} [startAt] GetDocumentsRequest startAt
                         * @property {boolean|null} [prove] GetDocumentsRequest prove
                         */

                        /**
                         * Constructs a new GetDocumentsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetDocumentsRequest.
                         * @implements IGetDocumentsRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest=} [properties] Properties to set
                         */
                        function GetDocumentsRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetDocumentsRequest dataContractId.
                         * @member {Uint8Array} dataContractId
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.dataContractId = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest documentType.
                         * @member {string} documentType
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.documentType = "";

                        /**
                         * GetDocumentsRequest where.
                         * @member {Uint8Array} where
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.where = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest orderBy.
                         * @member {Uint8Array} orderBy
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.orderBy = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest limit.
                         * @member {number} limit
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.limit = 0;

                        /**
                         * GetDocumentsRequest startAfter.
                         * @member {Uint8Array} startAfter
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.startAfter = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest startAt.
                         * @member {Uint8Array} startAt
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.startAt = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.prove = false;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * GetDocumentsRequest start.
                         * @member {"startAfter"|"startAt"|undefined} start
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        Object.defineProperty(GetDocumentsRequest.prototype, "start", {
                            get: $util.oneOfGetter($oneOfFields = ["startAfter", "startAt"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new GetDocumentsRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest instance
                         */
                        GetDocumentsRequest.create = function create(properties) {
                            return new GetDocumentsRequest(properties);
                        };

                        /**
                         * Encodes the specified GetDocumentsRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDocumentsRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} message GetDocumentsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDocumentsRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.dataContractId != null && Object.hasOwnProperty.call(message, "dataContractId"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.dataContractId);
                            if (message.documentType != null && Object.hasOwnProperty.call(message, "documentType"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.documentType);
                            if (message.where != null && Object.hasOwnProperty.call(message, "where"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.where);
                            if (message.orderBy != null && Object.hasOwnProperty.call(message, "orderBy"))
                                writer.uint32(/* id 4, wireType 2 =*/34).bytes(message.orderBy);
                            if (message.limit != null && Object.hasOwnProperty.call(message, "limit"))
                                writer.uint32(/* id 5, wireType 0 =*/40).uint32(message.limit);
                            if (message.startAfter != null && Object.hasOwnProperty.call(message, "startAfter"))
                                writer.uint32(/* id 6, wireType 2 =*/50).bytes(message.startAfter);
                            if (message.startAt != null && Object.hasOwnProperty.call(message, "startAt"))
                                writer.uint32(/* id 7, wireType 2 =*/58).bytes(message.startAt);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 8, wireType 0 =*/64).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetDocumentsRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDocumentsRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} message GetDocumentsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDocumentsRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetDocumentsRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDocumentsRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDocumentsRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.dataContractId = reader.bytes();
                                    break;
                                case 2:
                                    message.documentType = reader.string();
                                    break;
                                case 3:
                                    message.where = reader.bytes();
                                    break;
                                case 4:
                                    message.orderBy = reader.bytes();
                                    break;
                                case 5:
                                    message.limit = reader.uint32();
                                    break;
                                case 6:
                                    message.startAfter = reader.bytes();
                                    break;
                                case 7:
                                    message.startAt = reader.bytes();
                                    break;
                                case 8:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetDocumentsRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDocumentsRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetDocumentsRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetDocumentsRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.dataContractId != null && message.hasOwnProperty("dataContractId"))
                                if (!(message.dataContractId && typeof message.dataContractId.length === "number" || $util.isString(message.dataContractId)))
                                    return "dataContractId: buffer expected";
                            if (message.documentType != null && message.hasOwnProperty("documentType"))
                                if (!$util.isString(message.documentType))
                                    return "documentType: string expected";
                            if (message.where != null && message.hasOwnProperty("where"))
                                if (!(message.where && typeof message.where.length === "number" || $util.isString(message.where)))
                                    return "where: buffer expected";
                            if (message.orderBy != null && message.hasOwnProperty("orderBy"))
                                if (!(message.orderBy && typeof message.orderBy.length === "number" || $util.isString(message.orderBy)))
                                    return "orderBy: buffer expected";
                            if (message.limit != null && message.hasOwnProperty("limit"))
                                if (!$util.isInteger(message.limit))
                                    return "limit: integer expected";
                            if (message.startAfter != null && message.hasOwnProperty("startAfter")) {
                                properties.start = 1;
                                if (!(message.startAfter && typeof message.startAfter.length === "number" || $util.isString(message.startAfter)))
                                    return "startAfter: buffer expected";
                            }
                            if (message.startAt != null && message.hasOwnProperty("startAt")) {
                                if (properties.start === 1)
                                    return "start: multiple values";
                                properties.start = 1;
                                if (!(message.startAt && typeof message.startAt.length === "number" || $util.isString(message.startAt)))
                                    return "startAt: buffer expected";
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetDocumentsRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest
                         */
                        GetDocumentsRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetDocumentsRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetDocumentsRequest();
                            if (object.dataContractId != null)
                                if (typeof object.dataContractId === "string")
                                    $util.base64.decode(object.dataContractId, message.dataContractId = $util.newBuffer($util.base64.length(object.dataContractId)), 0);
                                else if (object.dataContractId.length >= 0)
                                    message.dataContractId = object.dataContractId;
                            if (object.documentType != null)
                                message.documentType = String(object.documentType);
                            if (object.where != null)
                                if (typeof object.where === "string")
                                    $util.base64.decode(object.where, message.where = $util.newBuffer($util.base64.length(object.where)), 0);
                                else if (object.where.length >= 0)
                                    message.where = object.where;
                            if (object.orderBy != null)
                                if (typeof object.orderBy === "string")
                                    $util.base64.decode(object.orderBy, message.orderBy = $util.newBuffer($util.base64.length(object.orderBy)), 0);
                                else if (object.orderBy.length >= 0)
                                    message.orderBy = object.orderBy;
                            if (object.limit != null)
                                message.limit = object.limit >>> 0;
                            if (object.startAfter != null)
                                if (typeof object.startAfter === "string")
                                    $util.base64.decode(object.startAfter, message.startAfter = $util.newBuffer($util.base64.length(object.startAfter)), 0);
                                else if (object.startAfter.length >= 0)
                                    message.startAfter = object.startAfter;
                            if (object.startAt != null)
                                if (typeof object.startAt === "string")
                                    $util.base64.decode(object.startAt, message.startAt = $util.newBuffer($util.base64.length(object.startAt)), 0);
                                else if (object.startAt.length >= 0)
                                    message.startAt = object.startAt;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetDocumentsRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetDocumentsRequest} message GetDocumentsRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetDocumentsRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.dataContractId = "";
                                else {
                                    object.dataContractId = [];
                                    if (options.bytes !== Array)
                                        object.dataContractId = $util.newBuffer(object.dataContractId);
                                }
                                object.documentType = "";
                                if (options.bytes === String)
                                    object.where = "";
                                else {
                                    object.where = [];
                                    if (options.bytes !== Array)
                                        object.where = $util.newBuffer(object.where);
                                }
                                if (options.bytes === String)
                                    object.orderBy = "";
                                else {
                                    object.orderBy = [];
                                    if (options.bytes !== Array)
                                        object.orderBy = $util.newBuffer(object.orderBy);
                                }
                                object.limit = 0;
                                object.prove = false;
                            }
                            if (message.dataContractId != null && message.hasOwnProperty("dataContractId"))
                                object.dataContractId = options.bytes === String ? $util.base64.encode(message.dataContractId, 0, message.dataContractId.length) : options.bytes === Array ? Array.prototype.slice.call(message.dataContractId) : message.dataContractId;
                            if (message.documentType != null && message.hasOwnProperty("documentType"))
                                object.documentType = message.documentType;
                            if (message.where != null && message.hasOwnProperty("where"))
                                object.where = options.bytes === String ? $util.base64.encode(message.where, 0, message.where.length) : options.bytes === Array ? Array.prototype.slice.call(message.where) : message.where;
                            if (message.orderBy != null && message.hasOwnProperty("orderBy"))
                                object.orderBy = options.bytes === String ? $util.base64.encode(message.orderBy, 0, message.orderBy.length) : options.bytes === Array ? Array.prototype.slice.call(message.orderBy) : message.orderBy;
                            if (message.limit != null && message.hasOwnProperty("limit"))
                                object.limit = message.limit;
                            if (message.startAfter != null && message.hasOwnProperty("startAfter")) {
                                object.startAfter = options.bytes === String ? $util.base64.encode(message.startAfter, 0, message.startAfter.length) : options.bytes === Array ? Array.prototype.slice.call(message.startAfter) : message.startAfter;
                                if (options.oneofs)
                                    object.start = "startAfter";
                            }
                            if (message.startAt != null && message.hasOwnProperty("startAt")) {
                                object.startAt = options.bytes === String ? $util.base64.encode(message.startAt, 0, message.startAt.length) : options.bytes === Array ? Array.prototype.slice.call(message.startAt) : message.startAt;
                                if (options.oneofs)
                                    object.start = "startAt";
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetDocumentsRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetDocumentsRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetDocumentsRequest;
                    })();

                    v0.GetDocumentsResponse = (function() {

                        /**
                         * Properties of a GetDocumentsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetDocumentsResponse
                         * @property {Array.<Uint8Array>|null} [documents] GetDocumentsResponse documents
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetDocumentsResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetDocumentsResponse metadata
                         */

                        /**
                         * Constructs a new GetDocumentsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetDocumentsResponse.
                         * @implements IGetDocumentsResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsResponse=} [properties] Properties to set
                         */
                        function GetDocumentsResponse(properties) {
                            this.documents = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetDocumentsResponse documents.
                         * @member {Array.<Uint8Array>} documents
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @instance
                         */
                        GetDocumentsResponse.prototype.documents = $util.emptyArray;

                        /**
                         * GetDocumentsResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @instance
                         */
                        GetDocumentsResponse.prototype.proof = null;

                        /**
                         * GetDocumentsResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @instance
                         */
                        GetDocumentsResponse.prototype.metadata = null;

                        /**
                         * Creates a new GetDocumentsResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsResponse} GetDocumentsResponse instance
                         */
                        GetDocumentsResponse.create = function create(properties) {
                            return new GetDocumentsResponse(properties);
                        };

                        /**
                         * Encodes the specified GetDocumentsResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDocumentsResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsResponse} message GetDocumentsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDocumentsResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.documents != null && message.documents.length)
                                for (var i = 0; i < message.documents.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.documents[i]);
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetDocumentsResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDocumentsResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsResponse} message GetDocumentsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDocumentsResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetDocumentsResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsResponse} GetDocumentsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDocumentsResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDocumentsResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.documents && message.documents.length))
                                        message.documents = [];
                                    message.documents.push(reader.bytes());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetDocumentsResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsResponse} GetDocumentsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDocumentsResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetDocumentsResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetDocumentsResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.documents != null && message.hasOwnProperty("documents")) {
                                if (!Array.isArray(message.documents))
                                    return "documents: array expected";
                                for (var i = 0; i < message.documents.length; ++i)
                                    if (!(message.documents[i] && typeof message.documents[i].length === "number" || $util.isString(message.documents[i])))
                                        return "documents: buffer[] expected";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                if (error)
                                    return "proof." + error;
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetDocumentsResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsResponse} GetDocumentsResponse
                         */
                        GetDocumentsResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetDocumentsResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetDocumentsResponse();
                            if (object.documents) {
                                if (!Array.isArray(object.documents))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDocumentsResponse.documents: array expected");
                                message.documents = [];
                                for (var i = 0; i < object.documents.length; ++i)
                                    if (typeof object.documents[i] === "string")
                                        $util.base64.decode(object.documents[i], message.documents[i] = $util.newBuffer($util.base64.length(object.documents[i])), 0);
                                    else if (object.documents[i].length >= 0)
                                        message.documents[i] = object.documents[i];
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDocumentsResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetDocumentsResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetDocumentsResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetDocumentsResponse} message GetDocumentsResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetDocumentsResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.documents = [];
                            if (options.defaults) {
                                object.proof = null;
                                object.metadata = null;
                            }
                            if (message.documents && message.documents.length) {
                                object.documents = [];
                                for (var j = 0; j < message.documents.length; ++j)
                                    object.documents[j] = options.bytes === String ? $util.base64.encode(message.documents[j], 0, message.documents[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.documents[j]) : message.documents[j];
                            }
                            if (message.proof != null && message.hasOwnProperty("proof"))
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetDocumentsResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetDocumentsResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetDocumentsResponse;
                    })();

                    v0.GetIdentitiesByPublicKeyHashesRequest = (function() {

                        /**
                         * Properties of a GetIdentitiesByPublicKeyHashesRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentitiesByPublicKeyHashesRequest
                         * @property {Array.<Uint8Array>|null} [publicKeyHashes] GetIdentitiesByPublicKeyHashesRequest publicKeyHashes
                         * @property {boolean|null} [prove] GetIdentitiesByPublicKeyHashesRequest prove
                         */

                        /**
                         * Constructs a new GetIdentitiesByPublicKeyHashesRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentitiesByPublicKeyHashesRequest.
                         * @implements IGetIdentitiesByPublicKeyHashesRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesRequest=} [properties] Properties to set
                         */
                        function GetIdentitiesByPublicKeyHashesRequest(properties) {
                            this.publicKeyHashes = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentitiesByPublicKeyHashesRequest publicKeyHashes.
                         * @member {Array.<Uint8Array>} publicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @instance
                         */
                        GetIdentitiesByPublicKeyHashesRequest.prototype.publicKeyHashes = $util.emptyArray;

                        /**
                         * GetIdentitiesByPublicKeyHashesRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @instance
                         */
                        GetIdentitiesByPublicKeyHashesRequest.prototype.prove = false;

                        /**
                         * Creates a new GetIdentitiesByPublicKeyHashesRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} GetIdentitiesByPublicKeyHashesRequest instance
                         */
                        GetIdentitiesByPublicKeyHashesRequest.create = function create(properties) {
                            return new GetIdentitiesByPublicKeyHashesRequest(properties);
                        };

                        /**
                         * Encodes the specified GetIdentitiesByPublicKeyHashesRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesRequest} message GetIdentitiesByPublicKeyHashesRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesByPublicKeyHashesRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.publicKeyHashes != null && message.publicKeyHashes.length)
                                for (var i = 0; i < message.publicKeyHashes.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.publicKeyHashes[i]);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentitiesByPublicKeyHashesRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesRequest} message GetIdentitiesByPublicKeyHashesRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesByPublicKeyHashesRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentitiesByPublicKeyHashesRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} GetIdentitiesByPublicKeyHashesRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesByPublicKeyHashesRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.publicKeyHashes && message.publicKeyHashes.length))
                                        message.publicKeyHashes = [];
                                    message.publicKeyHashes.push(reader.bytes());
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentitiesByPublicKeyHashesRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} GetIdentitiesByPublicKeyHashesRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesByPublicKeyHashesRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentitiesByPublicKeyHashesRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentitiesByPublicKeyHashesRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.publicKeyHashes != null && message.hasOwnProperty("publicKeyHashes")) {
                                if (!Array.isArray(message.publicKeyHashes))
                                    return "publicKeyHashes: array expected";
                                for (var i = 0; i < message.publicKeyHashes.length; ++i)
                                    if (!(message.publicKeyHashes[i] && typeof message.publicKeyHashes[i].length === "number" || $util.isString(message.publicKeyHashes[i])))
                                        return "publicKeyHashes: buffer[] expected";
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetIdentitiesByPublicKeyHashesRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} GetIdentitiesByPublicKeyHashesRequest
                         */
                        GetIdentitiesByPublicKeyHashesRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest();
                            if (object.publicKeyHashes) {
                                if (!Array.isArray(object.publicKeyHashes))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest.publicKeyHashes: array expected");
                                message.publicKeyHashes = [];
                                for (var i = 0; i < object.publicKeyHashes.length; ++i)
                                    if (typeof object.publicKeyHashes[i] === "string")
                                        $util.base64.decode(object.publicKeyHashes[i], message.publicKeyHashes[i] = $util.newBuffer($util.base64.length(object.publicKeyHashes[i])), 0);
                                    else if (object.publicKeyHashes[i].length >= 0)
                                        message.publicKeyHashes[i] = object.publicKeyHashes[i];
                            }
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentitiesByPublicKeyHashesRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest} message GetIdentitiesByPublicKeyHashesRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentitiesByPublicKeyHashesRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.publicKeyHashes = [];
                            if (options.defaults)
                                object.prove = false;
                            if (message.publicKeyHashes && message.publicKeyHashes.length) {
                                object.publicKeyHashes = [];
                                for (var j = 0; j < message.publicKeyHashes.length; ++j)
                                    object.publicKeyHashes[j] = options.bytes === String ? $util.base64.encode(message.publicKeyHashes[j], 0, message.publicKeyHashes[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.publicKeyHashes[j]) : message.publicKeyHashes[j];
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetIdentitiesByPublicKeyHashesRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentitiesByPublicKeyHashesRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentitiesByPublicKeyHashesRequest;
                    })();

                    v0.GetIdentitiesByPublicKeyHashesResponse = (function() {

                        /**
                         * Properties of a GetIdentitiesByPublicKeyHashesResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentitiesByPublicKeyHashesResponse
                         * @property {Array.<Uint8Array>|null} [identities] GetIdentitiesByPublicKeyHashesResponse identities
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentitiesByPublicKeyHashesResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentitiesByPublicKeyHashesResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentitiesByPublicKeyHashesResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentitiesByPublicKeyHashesResponse.
                         * @implements IGetIdentitiesByPublicKeyHashesResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesResponse=} [properties] Properties to set
                         */
                        function GetIdentitiesByPublicKeyHashesResponse(properties) {
                            this.identities = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentitiesByPublicKeyHashesResponse identities.
                         * @member {Array.<Uint8Array>} identities
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentitiesByPublicKeyHashesResponse.prototype.identities = $util.emptyArray;

                        /**
                         * GetIdentitiesByPublicKeyHashesResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentitiesByPublicKeyHashesResponse.prototype.proof = null;

                        /**
                         * GetIdentitiesByPublicKeyHashesResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentitiesByPublicKeyHashesResponse.prototype.metadata = null;

                        /**
                         * Creates a new GetIdentitiesByPublicKeyHashesResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} GetIdentitiesByPublicKeyHashesResponse instance
                         */
                        GetIdentitiesByPublicKeyHashesResponse.create = function create(properties) {
                            return new GetIdentitiesByPublicKeyHashesResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentitiesByPublicKeyHashesResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesResponse} message GetIdentitiesByPublicKeyHashesResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesByPublicKeyHashesResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.identities != null && message.identities.length)
                                for (var i = 0; i < message.identities.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.identities[i]);
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentitiesByPublicKeyHashesResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentitiesByPublicKeyHashesResponse} message GetIdentitiesByPublicKeyHashesResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentitiesByPublicKeyHashesResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentitiesByPublicKeyHashesResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} GetIdentitiesByPublicKeyHashesResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesByPublicKeyHashesResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.identities && message.identities.length))
                                        message.identities = [];
                                    message.identities.push(reader.bytes());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentitiesByPublicKeyHashesResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} GetIdentitiesByPublicKeyHashesResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentitiesByPublicKeyHashesResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentitiesByPublicKeyHashesResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentitiesByPublicKeyHashesResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.identities != null && message.hasOwnProperty("identities")) {
                                if (!Array.isArray(message.identities))
                                    return "identities: array expected";
                                for (var i = 0; i < message.identities.length; ++i)
                                    if (!(message.identities[i] && typeof message.identities[i].length === "number" || $util.isString(message.identities[i])))
                                        return "identities: buffer[] expected";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                if (error)
                                    return "proof." + error;
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetIdentitiesByPublicKeyHashesResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} GetIdentitiesByPublicKeyHashesResponse
                         */
                        GetIdentitiesByPublicKeyHashesResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse();
                            if (object.identities) {
                                if (!Array.isArray(object.identities))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.identities: array expected");
                                message.identities = [];
                                for (var i = 0; i < object.identities.length; ++i)
                                    if (typeof object.identities[i] === "string")
                                        $util.base64.decode(object.identities[i], message.identities[i] = $util.newBuffer($util.base64.length(object.identities[i])), 0);
                                    else if (object.identities[i].length >= 0)
                                        message.identities[i] = object.identities[i];
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentitiesByPublicKeyHashesResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse} message GetIdentitiesByPublicKeyHashesResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentitiesByPublicKeyHashesResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.identities = [];
                            if (options.defaults) {
                                object.proof = null;
                                object.metadata = null;
                            }
                            if (message.identities && message.identities.length) {
                                object.identities = [];
                                for (var j = 0; j < message.identities.length; ++j)
                                    object.identities[j] = options.bytes === String ? $util.base64.encode(message.identities[j], 0, message.identities[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.identities[j]) : message.identities[j];
                            }
                            if (message.proof != null && message.hasOwnProperty("proof"))
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentitiesByPublicKeyHashesResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentitiesByPublicKeyHashesResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentitiesByPublicKeyHashesResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentitiesByPublicKeyHashesResponse;
                    })();

                    v0.GetIdentityByPublicKeyHashesRequest = (function() {

                        /**
                         * Properties of a GetIdentityByPublicKeyHashesRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityByPublicKeyHashesRequest
                         * @property {Uint8Array|null} [publicKeyHash] GetIdentityByPublicKeyHashesRequest publicKeyHash
                         * @property {boolean|null} [prove] GetIdentityByPublicKeyHashesRequest prove
                         */

                        /**
                         * Constructs a new GetIdentityByPublicKeyHashesRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityByPublicKeyHashesRequest.
                         * @implements IGetIdentityByPublicKeyHashesRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesRequest=} [properties] Properties to set
                         */
                        function GetIdentityByPublicKeyHashesRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityByPublicKeyHashesRequest publicKeyHash.
                         * @member {Uint8Array} publicKeyHash
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @instance
                         */
                        GetIdentityByPublicKeyHashesRequest.prototype.publicKeyHash = $util.newBuffer([]);

                        /**
                         * GetIdentityByPublicKeyHashesRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @instance
                         */
                        GetIdentityByPublicKeyHashesRequest.prototype.prove = false;

                        /**
                         * Creates a new GetIdentityByPublicKeyHashesRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} GetIdentityByPublicKeyHashesRequest instance
                         */
                        GetIdentityByPublicKeyHashesRequest.create = function create(properties) {
                            return new GetIdentityByPublicKeyHashesRequest(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityByPublicKeyHashesRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesRequest} message GetIdentityByPublicKeyHashesRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityByPublicKeyHashesRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.publicKeyHash != null && Object.hasOwnProperty.call(message, "publicKeyHash"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.publicKeyHash);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityByPublicKeyHashesRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesRequest} message GetIdentityByPublicKeyHashesRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityByPublicKeyHashesRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityByPublicKeyHashesRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} GetIdentityByPublicKeyHashesRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityByPublicKeyHashesRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.publicKeyHash = reader.bytes();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityByPublicKeyHashesRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} GetIdentityByPublicKeyHashesRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityByPublicKeyHashesRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityByPublicKeyHashesRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityByPublicKeyHashesRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.publicKeyHash != null && message.hasOwnProperty("publicKeyHash"))
                                if (!(message.publicKeyHash && typeof message.publicKeyHash.length === "number" || $util.isString(message.publicKeyHash)))
                                    return "publicKeyHash: buffer expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetIdentityByPublicKeyHashesRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} GetIdentityByPublicKeyHashesRequest
                         */
                        GetIdentityByPublicKeyHashesRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest();
                            if (object.publicKeyHash != null)
                                if (typeof object.publicKeyHash === "string")
                                    $util.base64.decode(object.publicKeyHash, message.publicKeyHash = $util.newBuffer($util.base64.length(object.publicKeyHash)), 0);
                                else if (object.publicKeyHash.length >= 0)
                                    message.publicKeyHash = object.publicKeyHash;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityByPublicKeyHashesRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest} message GetIdentityByPublicKeyHashesRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityByPublicKeyHashesRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.publicKeyHash = "";
                                else {
                                    object.publicKeyHash = [];
                                    if (options.bytes !== Array)
                                        object.publicKeyHash = $util.newBuffer(object.publicKeyHash);
                                }
                                object.prove = false;
                            }
                            if (message.publicKeyHash != null && message.hasOwnProperty("publicKeyHash"))
                                object.publicKeyHash = options.bytes === String ? $util.base64.encode(message.publicKeyHash, 0, message.publicKeyHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.publicKeyHash) : message.publicKeyHash;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetIdentityByPublicKeyHashesRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityByPublicKeyHashesRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityByPublicKeyHashesRequest;
                    })();

                    v0.GetIdentityByPublicKeyHashesResponse = (function() {

                        /**
                         * Properties of a GetIdentityByPublicKeyHashesResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityByPublicKeyHashesResponse
                         * @property {Uint8Array|null} [identity] GetIdentityByPublicKeyHashesResponse identity
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentityByPublicKeyHashesResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentityByPublicKeyHashesResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentityByPublicKeyHashesResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityByPublicKeyHashesResponse.
                         * @implements IGetIdentityByPublicKeyHashesResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesResponse=} [properties] Properties to set
                         */
                        function GetIdentityByPublicKeyHashesResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityByPublicKeyHashesResponse identity.
                         * @member {Uint8Array} identity
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentityByPublicKeyHashesResponse.prototype.identity = $util.newBuffer([]);

                        /**
                         * GetIdentityByPublicKeyHashesResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentityByPublicKeyHashesResponse.prototype.proof = null;

                        /**
                         * GetIdentityByPublicKeyHashesResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentityByPublicKeyHashesResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * GetIdentityByPublicKeyHashesResponse result.
                         * @member {"identity"|"proof"|undefined} result
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @instance
                         */
                        Object.defineProperty(GetIdentityByPublicKeyHashesResponse.prototype, "result", {
                            get: $util.oneOfGetter($oneOfFields = ["identity", "proof"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new GetIdentityByPublicKeyHashesResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} GetIdentityByPublicKeyHashesResponse instance
                         */
                        GetIdentityByPublicKeyHashesResponse.create = function create(properties) {
                            return new GetIdentityByPublicKeyHashesResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityByPublicKeyHashesResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesResponse} message GetIdentityByPublicKeyHashesResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityByPublicKeyHashesResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.identity != null && Object.hasOwnProperty.call(message, "identity"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.identity);
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityByPublicKeyHashesResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityByPublicKeyHashesResponse} message GetIdentityByPublicKeyHashesResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityByPublicKeyHashesResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityByPublicKeyHashesResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} GetIdentityByPublicKeyHashesResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityByPublicKeyHashesResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.identity = reader.bytes();
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetIdentityByPublicKeyHashesResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} GetIdentityByPublicKeyHashesResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityByPublicKeyHashesResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityByPublicKeyHashesResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityByPublicKeyHashesResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.identity != null && message.hasOwnProperty("identity")) {
                                properties.result = 1;
                                if (!(message.identity && typeof message.identity.length === "number" || $util.isString(message.identity)))
                                    return "identity: buffer expected";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                if (properties.result === 1)
                                    return "result: multiple values";
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                    if (error)
                                        return "proof." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetIdentityByPublicKeyHashesResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} GetIdentityByPublicKeyHashesResponse
                         */
                        GetIdentityByPublicKeyHashesResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse();
                            if (object.identity != null)
                                if (typeof object.identity === "string")
                                    $util.base64.decode(object.identity, message.identity = $util.newBuffer($util.base64.length(object.identity)), 0);
                                else if (object.identity.length >= 0)
                                    message.identity = object.identity;
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityByPublicKeyHashesResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse} message GetIdentityByPublicKeyHashesResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityByPublicKeyHashesResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.identity != null && message.hasOwnProperty("identity")) {
                                object.identity = options.bytes === String ? $util.base64.encode(message.identity, 0, message.identity.length) : options.bytes === Array ? Array.prototype.slice.call(message.identity) : message.identity;
                                if (options.oneofs)
                                    object.result = "identity";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                                if (options.oneofs)
                                    object.result = "proof";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentityByPublicKeyHashesResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityByPublicKeyHashesResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityByPublicKeyHashesResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityByPublicKeyHashesResponse;
                    })();

                    v0.WaitForStateTransitionResultRequest = (function() {

                        /**
                         * Properties of a WaitForStateTransitionResultRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IWaitForStateTransitionResultRequest
                         * @property {Uint8Array|null} [stateTransitionHash] WaitForStateTransitionResultRequest stateTransitionHash
                         * @property {boolean|null} [prove] WaitForStateTransitionResultRequest prove
                         */

                        /**
                         * Constructs a new WaitForStateTransitionResultRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a WaitForStateTransitionResultRequest.
                         * @implements IWaitForStateTransitionResultRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest=} [properties] Properties to set
                         */
                        function WaitForStateTransitionResultRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * WaitForStateTransitionResultRequest stateTransitionHash.
                         * @member {Uint8Array} stateTransitionHash
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @instance
                         */
                        WaitForStateTransitionResultRequest.prototype.stateTransitionHash = $util.newBuffer([]);

                        /**
                         * WaitForStateTransitionResultRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @instance
                         */
                        WaitForStateTransitionResultRequest.prototype.prove = false;

                        /**
                         * Creates a new WaitForStateTransitionResultRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest instance
                         */
                        WaitForStateTransitionResultRequest.create = function create(properties) {
                            return new WaitForStateTransitionResultRequest(properties);
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} message WaitForStateTransitionResultRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.stateTransitionHash != null && Object.hasOwnProperty.call(message, "stateTransitionHash"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.stateTransitionHash);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} message WaitForStateTransitionResultRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.stateTransitionHash = reader.bytes();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a WaitForStateTransitionResultRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        WaitForStateTransitionResultRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.stateTransitionHash != null && message.hasOwnProperty("stateTransitionHash"))
                                if (!(message.stateTransitionHash && typeof message.stateTransitionHash.length === "number" || $util.isString(message.stateTransitionHash)))
                                    return "stateTransitionHash: buffer expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a WaitForStateTransitionResultRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest
                         */
                        WaitForStateTransitionResultRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest();
                            if (object.stateTransitionHash != null)
                                if (typeof object.stateTransitionHash === "string")
                                    $util.base64.decode(object.stateTransitionHash, message.stateTransitionHash = $util.newBuffer($util.base64.length(object.stateTransitionHash)), 0);
                                else if (object.stateTransitionHash.length >= 0)
                                    message.stateTransitionHash = object.stateTransitionHash;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a WaitForStateTransitionResultRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} message WaitForStateTransitionResultRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        WaitForStateTransitionResultRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.stateTransitionHash = "";
                                else {
                                    object.stateTransitionHash = [];
                                    if (options.bytes !== Array)
                                        object.stateTransitionHash = $util.newBuffer(object.stateTransitionHash);
                                }
                                object.prove = false;
                            }
                            if (message.stateTransitionHash != null && message.hasOwnProperty("stateTransitionHash"))
                                object.stateTransitionHash = options.bytes === String ? $util.base64.encode(message.stateTransitionHash, 0, message.stateTransitionHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.stateTransitionHash) : message.stateTransitionHash;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this WaitForStateTransitionResultRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        WaitForStateTransitionResultRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return WaitForStateTransitionResultRequest;
                    })();

                    v0.WaitForStateTransitionResultResponse = (function() {

                        /**
                         * Properties of a WaitForStateTransitionResultResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IWaitForStateTransitionResultResponse
                         * @property {org.dash.platform.dapi.v0.IStateTransitionBroadcastError|null} [error] WaitForStateTransitionResultResponse error
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] WaitForStateTransitionResultResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] WaitForStateTransitionResultResponse metadata
                         */

                        /**
                         * Constructs a new WaitForStateTransitionResultResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a WaitForStateTransitionResultResponse.
                         * @implements IWaitForStateTransitionResultResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse=} [properties] Properties to set
                         */
                        function WaitForStateTransitionResultResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * WaitForStateTransitionResultResponse error.
                         * @member {org.dash.platform.dapi.v0.IStateTransitionBroadcastError|null|undefined} error
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        WaitForStateTransitionResultResponse.prototype.error = null;

                        /**
                         * WaitForStateTransitionResultResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        WaitForStateTransitionResultResponse.prototype.proof = null;

                        /**
                         * WaitForStateTransitionResultResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        WaitForStateTransitionResultResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * WaitForStateTransitionResultResponse responses.
                         * @member {"error"|"proof"|undefined} responses
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        Object.defineProperty(WaitForStateTransitionResultResponse.prototype, "responses", {
                            get: $util.oneOfGetter($oneOfFields = ["error", "proof"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new WaitForStateTransitionResultResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse instance
                         */
                        WaitForStateTransitionResultResponse.create = function create(properties) {
                            return new WaitForStateTransitionResultResponse(properties);
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse} message WaitForStateTransitionResultResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.error != null && Object.hasOwnProperty.call(message, "error"))
                                $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.encode(message.error, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse} message WaitForStateTransitionResultResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.Proof.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a WaitForStateTransitionResultResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        WaitForStateTransitionResultResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.error != null && message.hasOwnProperty("error")) {
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.verify(message.error);
                                    if (error)
                                        return "error." + error;
                                }
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                if (properties.responses === 1)
                                    return "responses: multiple values";
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.Proof.verify(message.proof);
                                    if (error)
                                        return "proof." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a WaitForStateTransitionResultResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse
                         */
                        WaitForStateTransitionResultResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse();
                            if (object.error != null) {
                                if (typeof object.error !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.error: object expected");
                                message.error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.fromObject(object.error);
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a WaitForStateTransitionResultResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} message WaitForStateTransitionResultResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        WaitForStateTransitionResultResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.error != null && message.hasOwnProperty("error")) {
                                object.error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.toObject(message.error, options);
                                if (options.oneofs)
                                    object.responses = "error";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                                if (options.oneofs)
                                    object.responses = "proof";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this WaitForStateTransitionResultResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        WaitForStateTransitionResultResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return WaitForStateTransitionResultResponse;
                    })();

                    v0.ConsensusParamsBlock = (function() {

                        /**
                         * Properties of a ConsensusParamsBlock.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IConsensusParamsBlock
                         * @property {string|null} [maxBytes] ConsensusParamsBlock maxBytes
                         * @property {string|null} [maxGas] ConsensusParamsBlock maxGas
                         * @property {string|null} [timeIotaMs] ConsensusParamsBlock timeIotaMs
                         */

                        /**
                         * Constructs a new ConsensusParamsBlock.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ConsensusParamsBlock.
                         * @implements IConsensusParamsBlock
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock=} [properties] Properties to set
                         */
                        function ConsensusParamsBlock(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ConsensusParamsBlock maxBytes.
                         * @member {string} maxBytes
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         */
                        ConsensusParamsBlock.prototype.maxBytes = "";

                        /**
                         * ConsensusParamsBlock maxGas.
                         * @member {string} maxGas
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         */
                        ConsensusParamsBlock.prototype.maxGas = "";

                        /**
                         * ConsensusParamsBlock timeIotaMs.
                         * @member {string} timeIotaMs
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         */
                        ConsensusParamsBlock.prototype.timeIotaMs = "";

                        /**
                         * Creates a new ConsensusParamsBlock instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock instance
                         */
                        ConsensusParamsBlock.create = function create(properties) {
                            return new ConsensusParamsBlock(properties);
                        };

                        /**
                         * Encodes the specified ConsensusParamsBlock message. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsBlock.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock} message ConsensusParamsBlock message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsBlock.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.maxBytes != null && Object.hasOwnProperty.call(message, "maxBytes"))
                                writer.uint32(/* id 1, wireType 2 =*/10).string(message.maxBytes);
                            if (message.maxGas != null && Object.hasOwnProperty.call(message, "maxGas"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.maxGas);
                            if (message.timeIotaMs != null && Object.hasOwnProperty.call(message, "timeIotaMs"))
                                writer.uint32(/* id 3, wireType 2 =*/26).string(message.timeIotaMs);
                            return writer;
                        };

                        /**
                         * Encodes the specified ConsensusParamsBlock message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsBlock.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock} message ConsensusParamsBlock message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsBlock.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ConsensusParamsBlock message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsBlock.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ConsensusParamsBlock();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.maxBytes = reader.string();
                                    break;
                                case 2:
                                    message.maxGas = reader.string();
                                    break;
                                case 3:
                                    message.timeIotaMs = reader.string();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ConsensusParamsBlock message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsBlock.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ConsensusParamsBlock message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ConsensusParamsBlock.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                if (!$util.isString(message.maxBytes))
                                    return "maxBytes: string expected";
                            if (message.maxGas != null && message.hasOwnProperty("maxGas"))
                                if (!$util.isString(message.maxGas))
                                    return "maxGas: string expected";
                            if (message.timeIotaMs != null && message.hasOwnProperty("timeIotaMs"))
                                if (!$util.isString(message.timeIotaMs))
                                    return "timeIotaMs: string expected";
                            return null;
                        };

                        /**
                         * Creates a ConsensusParamsBlock message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock
                         */
                        ConsensusParamsBlock.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ConsensusParamsBlock)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ConsensusParamsBlock();
                            if (object.maxBytes != null)
                                message.maxBytes = String(object.maxBytes);
                            if (object.maxGas != null)
                                message.maxGas = String(object.maxGas);
                            if (object.timeIotaMs != null)
                                message.timeIotaMs = String(object.timeIotaMs);
                            return message;
                        };

                        /**
                         * Creates a plain object from a ConsensusParamsBlock message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.ConsensusParamsBlock} message ConsensusParamsBlock
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ConsensusParamsBlock.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.maxBytes = "";
                                object.maxGas = "";
                                object.timeIotaMs = "";
                            }
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                object.maxBytes = message.maxBytes;
                            if (message.maxGas != null && message.hasOwnProperty("maxGas"))
                                object.maxGas = message.maxGas;
                            if (message.timeIotaMs != null && message.hasOwnProperty("timeIotaMs"))
                                object.timeIotaMs = message.timeIotaMs;
                            return object;
                        };

                        /**
                         * Converts this ConsensusParamsBlock to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ConsensusParamsBlock.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ConsensusParamsBlock;
                    })();

                    v0.ConsensusParamsEvidence = (function() {

                        /**
                         * Properties of a ConsensusParamsEvidence.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IConsensusParamsEvidence
                         * @property {string|null} [maxAgeNumBlocks] ConsensusParamsEvidence maxAgeNumBlocks
                         * @property {string|null} [maxAgeDuration] ConsensusParamsEvidence maxAgeDuration
                         * @property {string|null} [maxBytes] ConsensusParamsEvidence maxBytes
                         */

                        /**
                         * Constructs a new ConsensusParamsEvidence.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ConsensusParamsEvidence.
                         * @implements IConsensusParamsEvidence
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence=} [properties] Properties to set
                         */
                        function ConsensusParamsEvidence(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ConsensusParamsEvidence maxAgeNumBlocks.
                         * @member {string} maxAgeNumBlocks
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         */
                        ConsensusParamsEvidence.prototype.maxAgeNumBlocks = "";

                        /**
                         * ConsensusParamsEvidence maxAgeDuration.
                         * @member {string} maxAgeDuration
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         */
                        ConsensusParamsEvidence.prototype.maxAgeDuration = "";

                        /**
                         * ConsensusParamsEvidence maxBytes.
                         * @member {string} maxBytes
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         */
                        ConsensusParamsEvidence.prototype.maxBytes = "";

                        /**
                         * Creates a new ConsensusParamsEvidence instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence instance
                         */
                        ConsensusParamsEvidence.create = function create(properties) {
                            return new ConsensusParamsEvidence(properties);
                        };

                        /**
                         * Encodes the specified ConsensusParamsEvidence message. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsEvidence.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence} message ConsensusParamsEvidence message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsEvidence.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.maxAgeNumBlocks != null && Object.hasOwnProperty.call(message, "maxAgeNumBlocks"))
                                writer.uint32(/* id 1, wireType 2 =*/10).string(message.maxAgeNumBlocks);
                            if (message.maxAgeDuration != null && Object.hasOwnProperty.call(message, "maxAgeDuration"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.maxAgeDuration);
                            if (message.maxBytes != null && Object.hasOwnProperty.call(message, "maxBytes"))
                                writer.uint32(/* id 3, wireType 2 =*/26).string(message.maxBytes);
                            return writer;
                        };

                        /**
                         * Encodes the specified ConsensusParamsEvidence message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsEvidence.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence} message ConsensusParamsEvidence message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsEvidence.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ConsensusParamsEvidence message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsEvidence.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.maxAgeNumBlocks = reader.string();
                                    break;
                                case 2:
                                    message.maxAgeDuration = reader.string();
                                    break;
                                case 3:
                                    message.maxBytes = reader.string();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ConsensusParamsEvidence message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsEvidence.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ConsensusParamsEvidence message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ConsensusParamsEvidence.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.maxAgeNumBlocks != null && message.hasOwnProperty("maxAgeNumBlocks"))
                                if (!$util.isString(message.maxAgeNumBlocks))
                                    return "maxAgeNumBlocks: string expected";
                            if (message.maxAgeDuration != null && message.hasOwnProperty("maxAgeDuration"))
                                if (!$util.isString(message.maxAgeDuration))
                                    return "maxAgeDuration: string expected";
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                if (!$util.isString(message.maxBytes))
                                    return "maxBytes: string expected";
                            return null;
                        };

                        /**
                         * Creates a ConsensusParamsEvidence message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence
                         */
                        ConsensusParamsEvidence.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence();
                            if (object.maxAgeNumBlocks != null)
                                message.maxAgeNumBlocks = String(object.maxAgeNumBlocks);
                            if (object.maxAgeDuration != null)
                                message.maxAgeDuration = String(object.maxAgeDuration);
                            if (object.maxBytes != null)
                                message.maxBytes = String(object.maxBytes);
                            return message;
                        };

                        /**
                         * Creates a plain object from a ConsensusParamsEvidence message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.ConsensusParamsEvidence} message ConsensusParamsEvidence
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ConsensusParamsEvidence.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.maxAgeNumBlocks = "";
                                object.maxAgeDuration = "";
                                object.maxBytes = "";
                            }
                            if (message.maxAgeNumBlocks != null && message.hasOwnProperty("maxAgeNumBlocks"))
                                object.maxAgeNumBlocks = message.maxAgeNumBlocks;
                            if (message.maxAgeDuration != null && message.hasOwnProperty("maxAgeDuration"))
                                object.maxAgeDuration = message.maxAgeDuration;
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                object.maxBytes = message.maxBytes;
                            return object;
                        };

                        /**
                         * Converts this ConsensusParamsEvidence to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ConsensusParamsEvidence.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ConsensusParamsEvidence;
                    })();

                    v0.GetConsensusParamsRequest = (function() {

                        /**
                         * Properties of a GetConsensusParamsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetConsensusParamsRequest
                         * @property {number|Long|null} [height] GetConsensusParamsRequest height
                         * @property {boolean|null} [prove] GetConsensusParamsRequest prove
                         */

                        /**
                         * Constructs a new GetConsensusParamsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetConsensusParamsRequest.
                         * @implements IGetConsensusParamsRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest=} [properties] Properties to set
                         */
                        function GetConsensusParamsRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetConsensusParamsRequest height.
                         * @member {number|Long} height
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @instance
                         */
                        GetConsensusParamsRequest.prototype.height = $util.Long ? $util.Long.fromBits(0,0,false) : 0;

                        /**
                         * GetConsensusParamsRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @instance
                         */
                        GetConsensusParamsRequest.prototype.prove = false;

                        /**
                         * Creates a new GetConsensusParamsRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest instance
                         */
                        GetConsensusParamsRequest.create = function create(properties) {
                            return new GetConsensusParamsRequest(properties);
                        };

                        /**
                         * Encodes the specified GetConsensusParamsRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} message GetConsensusParamsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.height != null && Object.hasOwnProperty.call(message, "height"))
                                writer.uint32(/* id 1, wireType 0 =*/8).int64(message.height);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetConsensusParamsRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} message GetConsensusParamsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetConsensusParamsRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.height = reader.int64();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetConsensusParamsRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetConsensusParamsRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetConsensusParamsRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (!$util.isInteger(message.height) && !(message.height && $util.isInteger(message.height.low) && $util.isInteger(message.height.high)))
                                    return "height: integer|Long expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetConsensusParamsRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest
                         */
                        GetConsensusParamsRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest();
                            if (object.height != null)
                                if ($util.Long)
                                    (message.height = $util.Long.fromValue(object.height)).unsigned = false;
                                else if (typeof object.height === "string")
                                    message.height = parseInt(object.height, 10);
                                else if (typeof object.height === "number")
                                    message.height = object.height;
                                else if (typeof object.height === "object")
                                    message.height = new $util.LongBits(object.height.low >>> 0, object.height.high >>> 0).toNumber();
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetConsensusParamsRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetConsensusParamsRequest} message GetConsensusParamsRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetConsensusParamsRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if ($util.Long) {
                                    var long = new $util.Long(0, 0, false);
                                    object.height = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                                } else
                                    object.height = options.longs === String ? "0" : 0;
                                object.prove = false;
                            }
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (typeof message.height === "number")
                                    object.height = options.longs === String ? String(message.height) : message.height;
                                else
                                    object.height = options.longs === String ? $util.Long.prototype.toString.call(message.height) : options.longs === Number ? new $util.LongBits(message.height.low >>> 0, message.height.high >>> 0).toNumber() : message.height;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetConsensusParamsRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetConsensusParamsRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetConsensusParamsRequest;
                    })();

                    v0.GetConsensusParamsResponse = (function() {

                        /**
                         * Properties of a GetConsensusParamsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetConsensusParamsResponse
                         * @property {org.dash.platform.dapi.v0.IConsensusParamsBlock|null} [block] GetConsensusParamsResponse block
                         * @property {org.dash.platform.dapi.v0.IConsensusParamsEvidence|null} [evidence] GetConsensusParamsResponse evidence
                         */

                        /**
                         * Constructs a new GetConsensusParamsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetConsensusParamsResponse.
                         * @implements IGetConsensusParamsResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse=} [properties] Properties to set
                         */
                        function GetConsensusParamsResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetConsensusParamsResponse block.
                         * @member {org.dash.platform.dapi.v0.IConsensusParamsBlock|null|undefined} block
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @instance
                         */
                        GetConsensusParamsResponse.prototype.block = null;

                        /**
                         * GetConsensusParamsResponse evidence.
                         * @member {org.dash.platform.dapi.v0.IConsensusParamsEvidence|null|undefined} evidence
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @instance
                         */
                        GetConsensusParamsResponse.prototype.evidence = null;

                        /**
                         * Creates a new GetConsensusParamsResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse instance
                         */
                        GetConsensusParamsResponse.create = function create(properties) {
                            return new GetConsensusParamsResponse(properties);
                        };

                        /**
                         * Encodes the specified GetConsensusParamsResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse} message GetConsensusParamsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.block != null && Object.hasOwnProperty.call(message, "block"))
                                $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.encode(message.block, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.evidence != null && Object.hasOwnProperty.call(message, "evidence"))
                                $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.encode(message.evidence, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetConsensusParamsResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse} message GetConsensusParamsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetConsensusParamsResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.block = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.evidence = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetConsensusParamsResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetConsensusParamsResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetConsensusParamsResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.block != null && message.hasOwnProperty("block")) {
                                var error = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.verify(message.block);
                                if (error)
                                    return "block." + error;
                            }
                            if (message.evidence != null && message.hasOwnProperty("evidence")) {
                                var error = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.verify(message.evidence);
                                if (error)
                                    return "evidence." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetConsensusParamsResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse
                         */
                        GetConsensusParamsResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse();
                            if (object.block != null) {
                                if (typeof object.block !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetConsensusParamsResponse.block: object expected");
                                message.block = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.fromObject(object.block);
                            }
                            if (object.evidence != null) {
                                if (typeof object.evidence !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetConsensusParamsResponse.evidence: object expected");
                                message.evidence = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.fromObject(object.evidence);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetConsensusParamsResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetConsensusParamsResponse} message GetConsensusParamsResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetConsensusParamsResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.block = null;
                                object.evidence = null;
                            }
                            if (message.block != null && message.hasOwnProperty("block"))
                                object.block = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.toObject(message.block, options);
                            if (message.evidence != null && message.hasOwnProperty("evidence"))
                                object.evidence = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.toObject(message.evidence, options);
                            return object;
                        };

                        /**
                         * Converts this GetConsensusParamsResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetConsensusParamsResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetConsensusParamsResponse;
                    })();

                    return v0;
                })();

                return dapi;
            })();

            return platform;
        })();

        return dash;
    })();

    return org;
})();

$root.google = (function() {

    /**
     * Namespace google.
     * @exports google
     * @namespace
     */
    var google = {};

    google.protobuf = (function() {

        /**
         * Namespace protobuf.
         * @memberof google
         * @namespace
         */
        var protobuf = {};

        protobuf.DoubleValue = (function() {

            /**
             * Properties of a DoubleValue.
             * @memberof google.protobuf
             * @interface IDoubleValue
             * @property {number|null} [value] DoubleValue value
             */

            /**
             * Constructs a new DoubleValue.
             * @memberof google.protobuf
             * @classdesc Represents a DoubleValue.
             * @implements IDoubleValue
             * @constructor
             * @param {google.protobuf.IDoubleValue=} [properties] Properties to set
             */
            function DoubleValue(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * DoubleValue value.
             * @member {number} value
             * @memberof google.protobuf.DoubleValue
             * @instance
             */
            DoubleValue.prototype.value = 0;

            /**
             * Creates a new DoubleValue instance using the specified properties.
             * @function create
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {google.protobuf.IDoubleValue=} [properties] Properties to set
             * @returns {google.protobuf.DoubleValue} DoubleValue instance
             */
            DoubleValue.create = function create(properties) {
                return new DoubleValue(properties);
            };

            /**
             * Encodes the specified DoubleValue message. Does not implicitly {@link google.protobuf.DoubleValue.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {google.protobuf.IDoubleValue} message DoubleValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            DoubleValue.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 1 =*/9).double(message.value);
                return writer;
            };

            /**
             * Encodes the specified DoubleValue message, length delimited. Does not implicitly {@link google.protobuf.DoubleValue.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {google.protobuf.IDoubleValue} message DoubleValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            DoubleValue.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a DoubleValue message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.DoubleValue} DoubleValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            DoubleValue.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.DoubleValue();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.double();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a DoubleValue message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.DoubleValue} DoubleValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            DoubleValue.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a DoubleValue message.
             * @function verify
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            DoubleValue.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (typeof message.value !== "number")
                        return "value: number expected";
                return null;
            };

            /**
             * Creates a DoubleValue message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.DoubleValue} DoubleValue
             */
            DoubleValue.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.DoubleValue)
                    return object;
                var message = new $root.google.protobuf.DoubleValue();
                if (object.value != null)
                    message.value = Number(object.value);
                return message;
            };

            /**
             * Creates a plain object from a DoubleValue message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.DoubleValue
             * @static
             * @param {google.protobuf.DoubleValue} message DoubleValue
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            DoubleValue.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    object.value = 0;
                if (message.value != null && message.hasOwnProperty("value"))
                    object.value = options.json && !isFinite(message.value) ? String(message.value) : message.value;
                return object;
            };

            /**
             * Converts this DoubleValue to JSON.
             * @function toJSON
             * @memberof google.protobuf.DoubleValue
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            DoubleValue.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return DoubleValue;
        })();

        protobuf.FloatValue = (function() {

            /**
             * Properties of a FloatValue.
             * @memberof google.protobuf
             * @interface IFloatValue
             * @property {number|null} [value] FloatValue value
             */

            /**
             * Constructs a new FloatValue.
             * @memberof google.protobuf
             * @classdesc Represents a FloatValue.
             * @implements IFloatValue
             * @constructor
             * @param {google.protobuf.IFloatValue=} [properties] Properties to set
             */
            function FloatValue(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * FloatValue value.
             * @member {number} value
             * @memberof google.protobuf.FloatValue
             * @instance
             */
            FloatValue.prototype.value = 0;

            /**
             * Creates a new FloatValue instance using the specified properties.
             * @function create
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {google.protobuf.IFloatValue=} [properties] Properties to set
             * @returns {google.protobuf.FloatValue} FloatValue instance
             */
            FloatValue.create = function create(properties) {
                return new FloatValue(properties);
            };

            /**
             * Encodes the specified FloatValue message. Does not implicitly {@link google.protobuf.FloatValue.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {google.protobuf.IFloatValue} message FloatValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            FloatValue.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 5 =*/13).float(message.value);
                return writer;
            };

            /**
             * Encodes the specified FloatValue message, length delimited. Does not implicitly {@link google.protobuf.FloatValue.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {google.protobuf.IFloatValue} message FloatValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            FloatValue.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a FloatValue message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.FloatValue} FloatValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            FloatValue.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.FloatValue();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.float();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a FloatValue message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.FloatValue} FloatValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            FloatValue.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a FloatValue message.
             * @function verify
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            FloatValue.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (typeof message.value !== "number")
                        return "value: number expected";
                return null;
            };

            /**
             * Creates a FloatValue message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.FloatValue} FloatValue
             */
            FloatValue.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.FloatValue)
                    return object;
                var message = new $root.google.protobuf.FloatValue();
                if (object.value != null)
                    message.value = Number(object.value);
                return message;
            };

            /**
             * Creates a plain object from a FloatValue message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.FloatValue
             * @static
             * @param {google.protobuf.FloatValue} message FloatValue
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            FloatValue.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    object.value = 0;
                if (message.value != null && message.hasOwnProperty("value"))
                    object.value = options.json && !isFinite(message.value) ? String(message.value) : message.value;
                return object;
            };

            /**
             * Converts this FloatValue to JSON.
             * @function toJSON
             * @memberof google.protobuf.FloatValue
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            FloatValue.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return FloatValue;
        })();

        protobuf.Int64Value = (function() {

            /**
             * Properties of an Int64Value.
             * @memberof google.protobuf
             * @interface IInt64Value
             * @property {number|Long|null} [value] Int64Value value
             */

            /**
             * Constructs a new Int64Value.
             * @memberof google.protobuf
             * @classdesc Represents an Int64Value.
             * @implements IInt64Value
             * @constructor
             * @param {google.protobuf.IInt64Value=} [properties] Properties to set
             */
            function Int64Value(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * Int64Value value.
             * @member {number|Long} value
             * @memberof google.protobuf.Int64Value
             * @instance
             */
            Int64Value.prototype.value = $util.Long ? $util.Long.fromBits(0,0,false) : 0;

            /**
             * Creates a new Int64Value instance using the specified properties.
             * @function create
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {google.protobuf.IInt64Value=} [properties] Properties to set
             * @returns {google.protobuf.Int64Value} Int64Value instance
             */
            Int64Value.create = function create(properties) {
                return new Int64Value(properties);
            };

            /**
             * Encodes the specified Int64Value message. Does not implicitly {@link google.protobuf.Int64Value.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {google.protobuf.IInt64Value} message Int64Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Int64Value.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 0 =*/8).int64(message.value);
                return writer;
            };

            /**
             * Encodes the specified Int64Value message, length delimited. Does not implicitly {@link google.protobuf.Int64Value.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {google.protobuf.IInt64Value} message Int64Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Int64Value.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes an Int64Value message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.Int64Value} Int64Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Int64Value.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.Int64Value();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.int64();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes an Int64Value message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.Int64Value} Int64Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Int64Value.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies an Int64Value message.
             * @function verify
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            Int64Value.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (!$util.isInteger(message.value) && !(message.value && $util.isInteger(message.value.low) && $util.isInteger(message.value.high)))
                        return "value: integer|Long expected";
                return null;
            };

            /**
             * Creates an Int64Value message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.Int64Value} Int64Value
             */
            Int64Value.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.Int64Value)
                    return object;
                var message = new $root.google.protobuf.Int64Value();
                if (object.value != null)
                    if ($util.Long)
                        (message.value = $util.Long.fromValue(object.value)).unsigned = false;
                    else if (typeof object.value === "string")
                        message.value = parseInt(object.value, 10);
                    else if (typeof object.value === "number")
                        message.value = object.value;
                    else if (typeof object.value === "object")
                        message.value = new $util.LongBits(object.value.low >>> 0, object.value.high >>> 0).toNumber();
                return message;
            };

            /**
             * Creates a plain object from an Int64Value message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.Int64Value
             * @static
             * @param {google.protobuf.Int64Value} message Int64Value
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            Int64Value.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    if ($util.Long) {
                        var long = new $util.Long(0, 0, false);
                        object.value = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                    } else
                        object.value = options.longs === String ? "0" : 0;
                if (message.value != null && message.hasOwnProperty("value"))
                    if (typeof message.value === "number")
                        object.value = options.longs === String ? String(message.value) : message.value;
                    else
                        object.value = options.longs === String ? $util.Long.prototype.toString.call(message.value) : options.longs === Number ? new $util.LongBits(message.value.low >>> 0, message.value.high >>> 0).toNumber() : message.value;
                return object;
            };

            /**
             * Converts this Int64Value to JSON.
             * @function toJSON
             * @memberof google.protobuf.Int64Value
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            Int64Value.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return Int64Value;
        })();

        protobuf.UInt64Value = (function() {

            /**
             * Properties of a UInt64Value.
             * @memberof google.protobuf
             * @interface IUInt64Value
             * @property {number|Long|null} [value] UInt64Value value
             */

            /**
             * Constructs a new UInt64Value.
             * @memberof google.protobuf
             * @classdesc Represents a UInt64Value.
             * @implements IUInt64Value
             * @constructor
             * @param {google.protobuf.IUInt64Value=} [properties] Properties to set
             */
            function UInt64Value(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * UInt64Value value.
             * @member {number|Long} value
             * @memberof google.protobuf.UInt64Value
             * @instance
             */
            UInt64Value.prototype.value = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

            /**
             * Creates a new UInt64Value instance using the specified properties.
             * @function create
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {google.protobuf.IUInt64Value=} [properties] Properties to set
             * @returns {google.protobuf.UInt64Value} UInt64Value instance
             */
            UInt64Value.create = function create(properties) {
                return new UInt64Value(properties);
            };

            /**
             * Encodes the specified UInt64Value message. Does not implicitly {@link google.protobuf.UInt64Value.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {google.protobuf.IUInt64Value} message UInt64Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            UInt64Value.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 0 =*/8).uint64(message.value);
                return writer;
            };

            /**
             * Encodes the specified UInt64Value message, length delimited. Does not implicitly {@link google.protobuf.UInt64Value.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {google.protobuf.IUInt64Value} message UInt64Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            UInt64Value.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a UInt64Value message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.UInt64Value} UInt64Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            UInt64Value.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.UInt64Value();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.uint64();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a UInt64Value message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.UInt64Value} UInt64Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            UInt64Value.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a UInt64Value message.
             * @function verify
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            UInt64Value.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (!$util.isInteger(message.value) && !(message.value && $util.isInteger(message.value.low) && $util.isInteger(message.value.high)))
                        return "value: integer|Long expected";
                return null;
            };

            /**
             * Creates a UInt64Value message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.UInt64Value} UInt64Value
             */
            UInt64Value.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.UInt64Value)
                    return object;
                var message = new $root.google.protobuf.UInt64Value();
                if (object.value != null)
                    if ($util.Long)
                        (message.value = $util.Long.fromValue(object.value)).unsigned = true;
                    else if (typeof object.value === "string")
                        message.value = parseInt(object.value, 10);
                    else if (typeof object.value === "number")
                        message.value = object.value;
                    else if (typeof object.value === "object")
                        message.value = new $util.LongBits(object.value.low >>> 0, object.value.high >>> 0).toNumber(true);
                return message;
            };

            /**
             * Creates a plain object from a UInt64Value message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.UInt64Value
             * @static
             * @param {google.protobuf.UInt64Value} message UInt64Value
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            UInt64Value.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    if ($util.Long) {
                        var long = new $util.Long(0, 0, true);
                        object.value = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                    } else
                        object.value = options.longs === String ? "0" : 0;
                if (message.value != null && message.hasOwnProperty("value"))
                    if (typeof message.value === "number")
                        object.value = options.longs === String ? String(message.value) : message.value;
                    else
                        object.value = options.longs === String ? $util.Long.prototype.toString.call(message.value) : options.longs === Number ? new $util.LongBits(message.value.low >>> 0, message.value.high >>> 0).toNumber(true) : message.value;
                return object;
            };

            /**
             * Converts this UInt64Value to JSON.
             * @function toJSON
             * @memberof google.protobuf.UInt64Value
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            UInt64Value.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return UInt64Value;
        })();

        protobuf.Int32Value = (function() {

            /**
             * Properties of an Int32Value.
             * @memberof google.protobuf
             * @interface IInt32Value
             * @property {number|null} [value] Int32Value value
             */

            /**
             * Constructs a new Int32Value.
             * @memberof google.protobuf
             * @classdesc Represents an Int32Value.
             * @implements IInt32Value
             * @constructor
             * @param {google.protobuf.IInt32Value=} [properties] Properties to set
             */
            function Int32Value(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * Int32Value value.
             * @member {number} value
             * @memberof google.protobuf.Int32Value
             * @instance
             */
            Int32Value.prototype.value = 0;

            /**
             * Creates a new Int32Value instance using the specified properties.
             * @function create
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {google.protobuf.IInt32Value=} [properties] Properties to set
             * @returns {google.protobuf.Int32Value} Int32Value instance
             */
            Int32Value.create = function create(properties) {
                return new Int32Value(properties);
            };

            /**
             * Encodes the specified Int32Value message. Does not implicitly {@link google.protobuf.Int32Value.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {google.protobuf.IInt32Value} message Int32Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Int32Value.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 0 =*/8).int32(message.value);
                return writer;
            };

            /**
             * Encodes the specified Int32Value message, length delimited. Does not implicitly {@link google.protobuf.Int32Value.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {google.protobuf.IInt32Value} message Int32Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Int32Value.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes an Int32Value message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.Int32Value} Int32Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Int32Value.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.Int32Value();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.int32();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes an Int32Value message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.Int32Value} Int32Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Int32Value.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies an Int32Value message.
             * @function verify
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            Int32Value.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (!$util.isInteger(message.value))
                        return "value: integer expected";
                return null;
            };

            /**
             * Creates an Int32Value message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.Int32Value} Int32Value
             */
            Int32Value.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.Int32Value)
                    return object;
                var message = new $root.google.protobuf.Int32Value();
                if (object.value != null)
                    message.value = object.value | 0;
                return message;
            };

            /**
             * Creates a plain object from an Int32Value message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.Int32Value
             * @static
             * @param {google.protobuf.Int32Value} message Int32Value
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            Int32Value.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    object.value = 0;
                if (message.value != null && message.hasOwnProperty("value"))
                    object.value = message.value;
                return object;
            };

            /**
             * Converts this Int32Value to JSON.
             * @function toJSON
             * @memberof google.protobuf.Int32Value
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            Int32Value.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return Int32Value;
        })();

        protobuf.UInt32Value = (function() {

            /**
             * Properties of a UInt32Value.
             * @memberof google.protobuf
             * @interface IUInt32Value
             * @property {number|null} [value] UInt32Value value
             */

            /**
             * Constructs a new UInt32Value.
             * @memberof google.protobuf
             * @classdesc Represents a UInt32Value.
             * @implements IUInt32Value
             * @constructor
             * @param {google.protobuf.IUInt32Value=} [properties] Properties to set
             */
            function UInt32Value(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * UInt32Value value.
             * @member {number} value
             * @memberof google.protobuf.UInt32Value
             * @instance
             */
            UInt32Value.prototype.value = 0;

            /**
             * Creates a new UInt32Value instance using the specified properties.
             * @function create
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {google.protobuf.IUInt32Value=} [properties] Properties to set
             * @returns {google.protobuf.UInt32Value} UInt32Value instance
             */
            UInt32Value.create = function create(properties) {
                return new UInt32Value(properties);
            };

            /**
             * Encodes the specified UInt32Value message. Does not implicitly {@link google.protobuf.UInt32Value.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {google.protobuf.IUInt32Value} message UInt32Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            UInt32Value.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.value);
                return writer;
            };

            /**
             * Encodes the specified UInt32Value message, length delimited. Does not implicitly {@link google.protobuf.UInt32Value.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {google.protobuf.IUInt32Value} message UInt32Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            UInt32Value.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a UInt32Value message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.UInt32Value} UInt32Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            UInt32Value.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.UInt32Value();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.uint32();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a UInt32Value message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.UInt32Value} UInt32Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            UInt32Value.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a UInt32Value message.
             * @function verify
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            UInt32Value.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (!$util.isInteger(message.value))
                        return "value: integer expected";
                return null;
            };

            /**
             * Creates a UInt32Value message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.UInt32Value} UInt32Value
             */
            UInt32Value.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.UInt32Value)
                    return object;
                var message = new $root.google.protobuf.UInt32Value();
                if (object.value != null)
                    message.value = object.value >>> 0;
                return message;
            };

            /**
             * Creates a plain object from a UInt32Value message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.UInt32Value
             * @static
             * @param {google.protobuf.UInt32Value} message UInt32Value
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            UInt32Value.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    object.value = 0;
                if (message.value != null && message.hasOwnProperty("value"))
                    object.value = message.value;
                return object;
            };

            /**
             * Converts this UInt32Value to JSON.
             * @function toJSON
             * @memberof google.protobuf.UInt32Value
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            UInt32Value.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return UInt32Value;
        })();

        protobuf.BoolValue = (function() {

            /**
             * Properties of a BoolValue.
             * @memberof google.protobuf
             * @interface IBoolValue
             * @property {boolean|null} [value] BoolValue value
             */

            /**
             * Constructs a new BoolValue.
             * @memberof google.protobuf
             * @classdesc Represents a BoolValue.
             * @implements IBoolValue
             * @constructor
             * @param {google.protobuf.IBoolValue=} [properties] Properties to set
             */
            function BoolValue(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * BoolValue value.
             * @member {boolean} value
             * @memberof google.protobuf.BoolValue
             * @instance
             */
            BoolValue.prototype.value = false;

            /**
             * Creates a new BoolValue instance using the specified properties.
             * @function create
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {google.protobuf.IBoolValue=} [properties] Properties to set
             * @returns {google.protobuf.BoolValue} BoolValue instance
             */
            BoolValue.create = function create(properties) {
                return new BoolValue(properties);
            };

            /**
             * Encodes the specified BoolValue message. Does not implicitly {@link google.protobuf.BoolValue.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {google.protobuf.IBoolValue} message BoolValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            BoolValue.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 0 =*/8).bool(message.value);
                return writer;
            };

            /**
             * Encodes the specified BoolValue message, length delimited. Does not implicitly {@link google.protobuf.BoolValue.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {google.protobuf.IBoolValue} message BoolValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            BoolValue.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a BoolValue message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.BoolValue} BoolValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            BoolValue.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.BoolValue();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.bool();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a BoolValue message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.BoolValue} BoolValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            BoolValue.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a BoolValue message.
             * @function verify
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            BoolValue.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (typeof message.value !== "boolean")
                        return "value: boolean expected";
                return null;
            };

            /**
             * Creates a BoolValue message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.BoolValue} BoolValue
             */
            BoolValue.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.BoolValue)
                    return object;
                var message = new $root.google.protobuf.BoolValue();
                if (object.value != null)
                    message.value = Boolean(object.value);
                return message;
            };

            /**
             * Creates a plain object from a BoolValue message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.BoolValue
             * @static
             * @param {google.protobuf.BoolValue} message BoolValue
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            BoolValue.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    object.value = false;
                if (message.value != null && message.hasOwnProperty("value"))
                    object.value = message.value;
                return object;
            };

            /**
             * Converts this BoolValue to JSON.
             * @function toJSON
             * @memberof google.protobuf.BoolValue
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            BoolValue.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return BoolValue;
        })();

        protobuf.StringValue = (function() {

            /**
             * Properties of a StringValue.
             * @memberof google.protobuf
             * @interface IStringValue
             * @property {string|null} [value] StringValue value
             */

            /**
             * Constructs a new StringValue.
             * @memberof google.protobuf
             * @classdesc Represents a StringValue.
             * @implements IStringValue
             * @constructor
             * @param {google.protobuf.IStringValue=} [properties] Properties to set
             */
            function StringValue(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * StringValue value.
             * @member {string} value
             * @memberof google.protobuf.StringValue
             * @instance
             */
            StringValue.prototype.value = "";

            /**
             * Creates a new StringValue instance using the specified properties.
             * @function create
             * @memberof google.protobuf.StringValue
             * @static
             * @param {google.protobuf.IStringValue=} [properties] Properties to set
             * @returns {google.protobuf.StringValue} StringValue instance
             */
            StringValue.create = function create(properties) {
                return new StringValue(properties);
            };

            /**
             * Encodes the specified StringValue message. Does not implicitly {@link google.protobuf.StringValue.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.StringValue
             * @static
             * @param {google.protobuf.IStringValue} message StringValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            StringValue.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 2 =*/10).string(message.value);
                return writer;
            };

            /**
             * Encodes the specified StringValue message, length delimited. Does not implicitly {@link google.protobuf.StringValue.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.StringValue
             * @static
             * @param {google.protobuf.IStringValue} message StringValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            StringValue.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a StringValue message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.StringValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.StringValue} StringValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            StringValue.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.StringValue();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.string();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a StringValue message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.StringValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.StringValue} StringValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            StringValue.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a StringValue message.
             * @function verify
             * @memberof google.protobuf.StringValue
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            StringValue.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (!$util.isString(message.value))
                        return "value: string expected";
                return null;
            };

            /**
             * Creates a StringValue message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.StringValue
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.StringValue} StringValue
             */
            StringValue.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.StringValue)
                    return object;
                var message = new $root.google.protobuf.StringValue();
                if (object.value != null)
                    message.value = String(object.value);
                return message;
            };

            /**
             * Creates a plain object from a StringValue message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.StringValue
             * @static
             * @param {google.protobuf.StringValue} message StringValue
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            StringValue.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    object.value = "";
                if (message.value != null && message.hasOwnProperty("value"))
                    object.value = message.value;
                return object;
            };

            /**
             * Converts this StringValue to JSON.
             * @function toJSON
             * @memberof google.protobuf.StringValue
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            StringValue.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return StringValue;
        })();

        protobuf.BytesValue = (function() {

            /**
             * Properties of a BytesValue.
             * @memberof google.protobuf
             * @interface IBytesValue
             * @property {Uint8Array|null} [value] BytesValue value
             */

            /**
             * Constructs a new BytesValue.
             * @memberof google.protobuf
             * @classdesc Represents a BytesValue.
             * @implements IBytesValue
             * @constructor
             * @param {google.protobuf.IBytesValue=} [properties] Properties to set
             */
            function BytesValue(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * BytesValue value.
             * @member {Uint8Array} value
             * @memberof google.protobuf.BytesValue
             * @instance
             */
            BytesValue.prototype.value = $util.newBuffer([]);

            /**
             * Creates a new BytesValue instance using the specified properties.
             * @function create
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {google.protobuf.IBytesValue=} [properties] Properties to set
             * @returns {google.protobuf.BytesValue} BytesValue instance
             */
            BytesValue.create = function create(properties) {
                return new BytesValue(properties);
            };

            /**
             * Encodes the specified BytesValue message. Does not implicitly {@link google.protobuf.BytesValue.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {google.protobuf.IBytesValue} message BytesValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            BytesValue.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.value);
                return writer;
            };

            /**
             * Encodes the specified BytesValue message, length delimited. Does not implicitly {@link google.protobuf.BytesValue.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {google.protobuf.IBytesValue} message BytesValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            BytesValue.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a BytesValue message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.BytesValue} BytesValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            BytesValue.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.BytesValue();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.value = reader.bytes();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a BytesValue message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.BytesValue} BytesValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            BytesValue.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a BytesValue message.
             * @function verify
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            BytesValue.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.value != null && message.hasOwnProperty("value"))
                    if (!(message.value && typeof message.value.length === "number" || $util.isString(message.value)))
                        return "value: buffer expected";
                return null;
            };

            /**
             * Creates a BytesValue message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.BytesValue} BytesValue
             */
            BytesValue.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.BytesValue)
                    return object;
                var message = new $root.google.protobuf.BytesValue();
                if (object.value != null)
                    if (typeof object.value === "string")
                        $util.base64.decode(object.value, message.value = $util.newBuffer($util.base64.length(object.value)), 0);
                    else if (object.value.length >= 0)
                        message.value = object.value;
                return message;
            };

            /**
             * Creates a plain object from a BytesValue message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.BytesValue
             * @static
             * @param {google.protobuf.BytesValue} message BytesValue
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            BytesValue.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults)
                    if (options.bytes === String)
                        object.value = "";
                    else {
                        object.value = [];
                        if (options.bytes !== Array)
                            object.value = $util.newBuffer(object.value);
                    }
                if (message.value != null && message.hasOwnProperty("value"))
                    object.value = options.bytes === String ? $util.base64.encode(message.value, 0, message.value.length) : options.bytes === Array ? Array.prototype.slice.call(message.value) : message.value;
                return object;
            };

            /**
             * Converts this BytesValue to JSON.
             * @function toJSON
             * @memberof google.protobuf.BytesValue
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            BytesValue.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return BytesValue;
        })();

        protobuf.Struct = (function() {

            /**
             * Properties of a Struct.
             * @memberof google.protobuf
             * @interface IStruct
             * @property {Object.<string,google.protobuf.IValue>|null} [fields] Struct fields
             */

            /**
             * Constructs a new Struct.
             * @memberof google.protobuf
             * @classdesc Represents a Struct.
             * @implements IStruct
             * @constructor
             * @param {google.protobuf.IStruct=} [properties] Properties to set
             */
            function Struct(properties) {
                this.fields = {};
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * Struct fields.
             * @member {Object.<string,google.protobuf.IValue>} fields
             * @memberof google.protobuf.Struct
             * @instance
             */
            Struct.prototype.fields = $util.emptyObject;

            /**
             * Creates a new Struct instance using the specified properties.
             * @function create
             * @memberof google.protobuf.Struct
             * @static
             * @param {google.protobuf.IStruct=} [properties] Properties to set
             * @returns {google.protobuf.Struct} Struct instance
             */
            Struct.create = function create(properties) {
                return new Struct(properties);
            };

            /**
             * Encodes the specified Struct message. Does not implicitly {@link google.protobuf.Struct.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.Struct
             * @static
             * @param {google.protobuf.IStruct} message Struct message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Struct.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.fields != null && Object.hasOwnProperty.call(message, "fields"))
                    for (var keys = Object.keys(message.fields), i = 0; i < keys.length; ++i) {
                        writer.uint32(/* id 1, wireType 2 =*/10).fork().uint32(/* id 1, wireType 2 =*/10).string(keys[i]);
                        $root.google.protobuf.Value.encode(message.fields[keys[i]], writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim().ldelim();
                    }
                return writer;
            };

            /**
             * Encodes the specified Struct message, length delimited. Does not implicitly {@link google.protobuf.Struct.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.Struct
             * @static
             * @param {google.protobuf.IStruct} message Struct message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Struct.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a Struct message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.Struct
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.Struct} Struct
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Struct.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.Struct(), key, value;
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        if (message.fields === $util.emptyObject)
                            message.fields = {};
                        var end2 = reader.uint32() + reader.pos;
                        key = "";
                        value = null;
                        while (reader.pos < end2) {
                            var tag2 = reader.uint32();
                            switch (tag2 >>> 3) {
                            case 1:
                                key = reader.string();
                                break;
                            case 2:
                                value = $root.google.protobuf.Value.decode(reader, reader.uint32());
                                break;
                            default:
                                reader.skipType(tag2 & 7);
                                break;
                            }
                        }
                        message.fields[key] = value;
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a Struct message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.Struct
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.Struct} Struct
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Struct.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a Struct message.
             * @function verify
             * @memberof google.protobuf.Struct
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            Struct.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.fields != null && message.hasOwnProperty("fields")) {
                    if (!$util.isObject(message.fields))
                        return "fields: object expected";
                    var key = Object.keys(message.fields);
                    for (var i = 0; i < key.length; ++i) {
                        var error = $root.google.protobuf.Value.verify(message.fields[key[i]]);
                        if (error)
                            return "fields." + error;
                    }
                }
                return null;
            };

            /**
             * Creates a Struct message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.Struct
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.Struct} Struct
             */
            Struct.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.Struct)
                    return object;
                var message = new $root.google.protobuf.Struct();
                if (object.fields) {
                    if (typeof object.fields !== "object")
                        throw TypeError(".google.protobuf.Struct.fields: object expected");
                    message.fields = {};
                    for (var keys = Object.keys(object.fields), i = 0; i < keys.length; ++i) {
                        if (typeof object.fields[keys[i]] !== "object")
                            throw TypeError(".google.protobuf.Struct.fields: object expected");
                        message.fields[keys[i]] = $root.google.protobuf.Value.fromObject(object.fields[keys[i]]);
                    }
                }
                return message;
            };

            /**
             * Creates a plain object from a Struct message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.Struct
             * @static
             * @param {google.protobuf.Struct} message Struct
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            Struct.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.objects || options.defaults)
                    object.fields = {};
                var keys2;
                if (message.fields && (keys2 = Object.keys(message.fields)).length) {
                    object.fields = {};
                    for (var j = 0; j < keys2.length; ++j)
                        object.fields[keys2[j]] = $root.google.protobuf.Value.toObject(message.fields[keys2[j]], options);
                }
                return object;
            };

            /**
             * Converts this Struct to JSON.
             * @function toJSON
             * @memberof google.protobuf.Struct
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            Struct.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return Struct;
        })();

        protobuf.Value = (function() {

            /**
             * Properties of a Value.
             * @memberof google.protobuf
             * @interface IValue
             * @property {google.protobuf.NullValue|null} [nullValue] Value nullValue
             * @property {number|null} [numberValue] Value numberValue
             * @property {string|null} [stringValue] Value stringValue
             * @property {boolean|null} [boolValue] Value boolValue
             * @property {google.protobuf.IStruct|null} [structValue] Value structValue
             * @property {google.protobuf.IListValue|null} [listValue] Value listValue
             */

            /**
             * Constructs a new Value.
             * @memberof google.protobuf
             * @classdesc Represents a Value.
             * @implements IValue
             * @constructor
             * @param {google.protobuf.IValue=} [properties] Properties to set
             */
            function Value(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * Value nullValue.
             * @member {google.protobuf.NullValue} nullValue
             * @memberof google.protobuf.Value
             * @instance
             */
            Value.prototype.nullValue = 0;

            /**
             * Value numberValue.
             * @member {number} numberValue
             * @memberof google.protobuf.Value
             * @instance
             */
            Value.prototype.numberValue = 0;

            /**
             * Value stringValue.
             * @member {string} stringValue
             * @memberof google.protobuf.Value
             * @instance
             */
            Value.prototype.stringValue = "";

            /**
             * Value boolValue.
             * @member {boolean} boolValue
             * @memberof google.protobuf.Value
             * @instance
             */
            Value.prototype.boolValue = false;

            /**
             * Value structValue.
             * @member {google.protobuf.IStruct|null|undefined} structValue
             * @memberof google.protobuf.Value
             * @instance
             */
            Value.prototype.structValue = null;

            /**
             * Value listValue.
             * @member {google.protobuf.IListValue|null|undefined} listValue
             * @memberof google.protobuf.Value
             * @instance
             */
            Value.prototype.listValue = null;

            // OneOf field names bound to virtual getters and setters
            var $oneOfFields;

            /**
             * Value kind.
             * @member {"nullValue"|"numberValue"|"stringValue"|"boolValue"|"structValue"|"listValue"|undefined} kind
             * @memberof google.protobuf.Value
             * @instance
             */
            Object.defineProperty(Value.prototype, "kind", {
                get: $util.oneOfGetter($oneOfFields = ["nullValue", "numberValue", "stringValue", "boolValue", "structValue", "listValue"]),
                set: $util.oneOfSetter($oneOfFields)
            });

            /**
             * Creates a new Value instance using the specified properties.
             * @function create
             * @memberof google.protobuf.Value
             * @static
             * @param {google.protobuf.IValue=} [properties] Properties to set
             * @returns {google.protobuf.Value} Value instance
             */
            Value.create = function create(properties) {
                return new Value(properties);
            };

            /**
             * Encodes the specified Value message. Does not implicitly {@link google.protobuf.Value.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.Value
             * @static
             * @param {google.protobuf.IValue} message Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Value.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.nullValue != null && Object.hasOwnProperty.call(message, "nullValue"))
                    writer.uint32(/* id 1, wireType 0 =*/8).int32(message.nullValue);
                if (message.numberValue != null && Object.hasOwnProperty.call(message, "numberValue"))
                    writer.uint32(/* id 2, wireType 1 =*/17).double(message.numberValue);
                if (message.stringValue != null && Object.hasOwnProperty.call(message, "stringValue"))
                    writer.uint32(/* id 3, wireType 2 =*/26).string(message.stringValue);
                if (message.boolValue != null && Object.hasOwnProperty.call(message, "boolValue"))
                    writer.uint32(/* id 4, wireType 0 =*/32).bool(message.boolValue);
                if (message.structValue != null && Object.hasOwnProperty.call(message, "structValue"))
                    $root.google.protobuf.Struct.encode(message.structValue, writer.uint32(/* id 5, wireType 2 =*/42).fork()).ldelim();
                if (message.listValue != null && Object.hasOwnProperty.call(message, "listValue"))
                    $root.google.protobuf.ListValue.encode(message.listValue, writer.uint32(/* id 6, wireType 2 =*/50).fork()).ldelim();
                return writer;
            };

            /**
             * Encodes the specified Value message, length delimited. Does not implicitly {@link google.protobuf.Value.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.Value
             * @static
             * @param {google.protobuf.IValue} message Value message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Value.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a Value message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.Value} Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Value.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.Value();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.nullValue = reader.int32();
                        break;
                    case 2:
                        message.numberValue = reader.double();
                        break;
                    case 3:
                        message.stringValue = reader.string();
                        break;
                    case 4:
                        message.boolValue = reader.bool();
                        break;
                    case 5:
                        message.structValue = $root.google.protobuf.Struct.decode(reader, reader.uint32());
                        break;
                    case 6:
                        message.listValue = $root.google.protobuf.ListValue.decode(reader, reader.uint32());
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a Value message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.Value
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.Value} Value
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Value.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a Value message.
             * @function verify
             * @memberof google.protobuf.Value
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            Value.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                var properties = {};
                if (message.nullValue != null && message.hasOwnProperty("nullValue")) {
                    properties.kind = 1;
                    switch (message.nullValue) {
                    default:
                        return "nullValue: enum value expected";
                    case 0:
                        break;
                    }
                }
                if (message.numberValue != null && message.hasOwnProperty("numberValue")) {
                    if (properties.kind === 1)
                        return "kind: multiple values";
                    properties.kind = 1;
                    if (typeof message.numberValue !== "number")
                        return "numberValue: number expected";
                }
                if (message.stringValue != null && message.hasOwnProperty("stringValue")) {
                    if (properties.kind === 1)
                        return "kind: multiple values";
                    properties.kind = 1;
                    if (!$util.isString(message.stringValue))
                        return "stringValue: string expected";
                }
                if (message.boolValue != null && message.hasOwnProperty("boolValue")) {
                    if (properties.kind === 1)
                        return "kind: multiple values";
                    properties.kind = 1;
                    if (typeof message.boolValue !== "boolean")
                        return "boolValue: boolean expected";
                }
                if (message.structValue != null && message.hasOwnProperty("structValue")) {
                    if (properties.kind === 1)
                        return "kind: multiple values";
                    properties.kind = 1;
                    {
                        var error = $root.google.protobuf.Struct.verify(message.structValue);
                        if (error)
                            return "structValue." + error;
                    }
                }
                if (message.listValue != null && message.hasOwnProperty("listValue")) {
                    if (properties.kind === 1)
                        return "kind: multiple values";
                    properties.kind = 1;
                    {
                        var error = $root.google.protobuf.ListValue.verify(message.listValue);
                        if (error)
                            return "listValue." + error;
                    }
                }
                return null;
            };

            /**
             * Creates a Value message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.Value
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.Value} Value
             */
            Value.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.Value)
                    return object;
                var message = new $root.google.protobuf.Value();
                switch (object.nullValue) {
                case "NULL_VALUE":
                case 0:
                    message.nullValue = 0;
                    break;
                }
                if (object.numberValue != null)
                    message.numberValue = Number(object.numberValue);
                if (object.stringValue != null)
                    message.stringValue = String(object.stringValue);
                if (object.boolValue != null)
                    message.boolValue = Boolean(object.boolValue);
                if (object.structValue != null) {
                    if (typeof object.structValue !== "object")
                        throw TypeError(".google.protobuf.Value.structValue: object expected");
                    message.structValue = $root.google.protobuf.Struct.fromObject(object.structValue);
                }
                if (object.listValue != null) {
                    if (typeof object.listValue !== "object")
                        throw TypeError(".google.protobuf.Value.listValue: object expected");
                    message.listValue = $root.google.protobuf.ListValue.fromObject(object.listValue);
                }
                return message;
            };

            /**
             * Creates a plain object from a Value message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.Value
             * @static
             * @param {google.protobuf.Value} message Value
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            Value.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (message.nullValue != null && message.hasOwnProperty("nullValue")) {
                    object.nullValue = options.enums === String ? $root.google.protobuf.NullValue[message.nullValue] : message.nullValue;
                    if (options.oneofs)
                        object.kind = "nullValue";
                }
                if (message.numberValue != null && message.hasOwnProperty("numberValue")) {
                    object.numberValue = options.json && !isFinite(message.numberValue) ? String(message.numberValue) : message.numberValue;
                    if (options.oneofs)
                        object.kind = "numberValue";
                }
                if (message.stringValue != null && message.hasOwnProperty("stringValue")) {
                    object.stringValue = message.stringValue;
                    if (options.oneofs)
                        object.kind = "stringValue";
                }
                if (message.boolValue != null && message.hasOwnProperty("boolValue")) {
                    object.boolValue = message.boolValue;
                    if (options.oneofs)
                        object.kind = "boolValue";
                }
                if (message.structValue != null && message.hasOwnProperty("structValue")) {
                    object.structValue = $root.google.protobuf.Struct.toObject(message.structValue, options);
                    if (options.oneofs)
                        object.kind = "structValue";
                }
                if (message.listValue != null && message.hasOwnProperty("listValue")) {
                    object.listValue = $root.google.protobuf.ListValue.toObject(message.listValue, options);
                    if (options.oneofs)
                        object.kind = "listValue";
                }
                return object;
            };

            /**
             * Converts this Value to JSON.
             * @function toJSON
             * @memberof google.protobuf.Value
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            Value.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return Value;
        })();

        /**
         * NullValue enum.
         * @name google.protobuf.NullValue
         * @enum {number}
         * @property {number} NULL_VALUE=0 NULL_VALUE value
         */
        protobuf.NullValue = (function() {
            var valuesById = {}, values = Object.create(valuesById);
            values[valuesById[0] = "NULL_VALUE"] = 0;
            return values;
        })();

        protobuf.ListValue = (function() {

            /**
             * Properties of a ListValue.
             * @memberof google.protobuf
             * @interface IListValue
             * @property {Array.<google.protobuf.IValue>|null} [values] ListValue values
             */

            /**
             * Constructs a new ListValue.
             * @memberof google.protobuf
             * @classdesc Represents a ListValue.
             * @implements IListValue
             * @constructor
             * @param {google.protobuf.IListValue=} [properties] Properties to set
             */
            function ListValue(properties) {
                this.values = [];
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * ListValue values.
             * @member {Array.<google.protobuf.IValue>} values
             * @memberof google.protobuf.ListValue
             * @instance
             */
            ListValue.prototype.values = $util.emptyArray;

            /**
             * Creates a new ListValue instance using the specified properties.
             * @function create
             * @memberof google.protobuf.ListValue
             * @static
             * @param {google.protobuf.IListValue=} [properties] Properties to set
             * @returns {google.protobuf.ListValue} ListValue instance
             */
            ListValue.create = function create(properties) {
                return new ListValue(properties);
            };

            /**
             * Encodes the specified ListValue message. Does not implicitly {@link google.protobuf.ListValue.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.ListValue
             * @static
             * @param {google.protobuf.IListValue} message ListValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            ListValue.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.values != null && message.values.length)
                    for (var i = 0; i < message.values.length; ++i)
                        $root.google.protobuf.Value.encode(message.values[i], writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                return writer;
            };

            /**
             * Encodes the specified ListValue message, length delimited. Does not implicitly {@link google.protobuf.ListValue.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.ListValue
             * @static
             * @param {google.protobuf.IListValue} message ListValue message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            ListValue.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a ListValue message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.ListValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.ListValue} ListValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            ListValue.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.ListValue();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        if (!(message.values && message.values.length))
                            message.values = [];
                        message.values.push($root.google.protobuf.Value.decode(reader, reader.uint32()));
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a ListValue message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.ListValue
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.ListValue} ListValue
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            ListValue.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a ListValue message.
             * @function verify
             * @memberof google.protobuf.ListValue
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            ListValue.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.values != null && message.hasOwnProperty("values")) {
                    if (!Array.isArray(message.values))
                        return "values: array expected";
                    for (var i = 0; i < message.values.length; ++i) {
                        var error = $root.google.protobuf.Value.verify(message.values[i]);
                        if (error)
                            return "values." + error;
                    }
                }
                return null;
            };

            /**
             * Creates a ListValue message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.ListValue
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.ListValue} ListValue
             */
            ListValue.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.ListValue)
                    return object;
                var message = new $root.google.protobuf.ListValue();
                if (object.values) {
                    if (!Array.isArray(object.values))
                        throw TypeError(".google.protobuf.ListValue.values: array expected");
                    message.values = [];
                    for (var i = 0; i < object.values.length; ++i) {
                        if (typeof object.values[i] !== "object")
                            throw TypeError(".google.protobuf.ListValue.values: object expected");
                        message.values[i] = $root.google.protobuf.Value.fromObject(object.values[i]);
                    }
                }
                return message;
            };

            /**
             * Creates a plain object from a ListValue message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.ListValue
             * @static
             * @param {google.protobuf.ListValue} message ListValue
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            ListValue.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.arrays || options.defaults)
                    object.values = [];
                if (message.values && message.values.length) {
                    object.values = [];
                    for (var j = 0; j < message.values.length; ++j)
                        object.values[j] = $root.google.protobuf.Value.toObject(message.values[j], options);
                }
                return object;
            };

            /**
             * Converts this ListValue to JSON.
             * @function toJSON
             * @memberof google.protobuf.ListValue
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            ListValue.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return ListValue;
        })();

        protobuf.Timestamp = (function() {

            /**
             * Properties of a Timestamp.
             * @memberof google.protobuf
             * @interface ITimestamp
             * @property {number|Long|null} [seconds] Timestamp seconds
             * @property {number|null} [nanos] Timestamp nanos
             */

            /**
             * Constructs a new Timestamp.
             * @memberof google.protobuf
             * @classdesc Represents a Timestamp.
             * @implements ITimestamp
             * @constructor
             * @param {google.protobuf.ITimestamp=} [properties] Properties to set
             */
            function Timestamp(properties) {
                if (properties)
                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                        if (properties[keys[i]] != null)
                            this[keys[i]] = properties[keys[i]];
            }

            /**
             * Timestamp seconds.
             * @member {number|Long} seconds
             * @memberof google.protobuf.Timestamp
             * @instance
             */
            Timestamp.prototype.seconds = $util.Long ? $util.Long.fromBits(0,0,false) : 0;

            /**
             * Timestamp nanos.
             * @member {number} nanos
             * @memberof google.protobuf.Timestamp
             * @instance
             */
            Timestamp.prototype.nanos = 0;

            /**
             * Creates a new Timestamp instance using the specified properties.
             * @function create
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {google.protobuf.ITimestamp=} [properties] Properties to set
             * @returns {google.protobuf.Timestamp} Timestamp instance
             */
            Timestamp.create = function create(properties) {
                return new Timestamp(properties);
            };

            /**
             * Encodes the specified Timestamp message. Does not implicitly {@link google.protobuf.Timestamp.verify|verify} messages.
             * @function encode
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {google.protobuf.ITimestamp} message Timestamp message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Timestamp.encode = function encode(message, writer) {
                if (!writer)
                    writer = $Writer.create();
                if (message.seconds != null && Object.hasOwnProperty.call(message, "seconds"))
                    writer.uint32(/* id 1, wireType 0 =*/8).int64(message.seconds);
                if (message.nanos != null && Object.hasOwnProperty.call(message, "nanos"))
                    writer.uint32(/* id 2, wireType 0 =*/16).int32(message.nanos);
                return writer;
            };

            /**
             * Encodes the specified Timestamp message, length delimited. Does not implicitly {@link google.protobuf.Timestamp.verify|verify} messages.
             * @function encodeDelimited
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {google.protobuf.ITimestamp} message Timestamp message or plain object to encode
             * @param {$protobuf.Writer} [writer] Writer to encode to
             * @returns {$protobuf.Writer} Writer
             */
            Timestamp.encodeDelimited = function encodeDelimited(message, writer) {
                return this.encode(message, writer).ldelim();
            };

            /**
             * Decodes a Timestamp message from the specified reader or buffer.
             * @function decode
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @param {number} [length] Message length if known beforehand
             * @returns {google.protobuf.Timestamp} Timestamp
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Timestamp.decode = function decode(reader, length) {
                if (!(reader instanceof $Reader))
                    reader = $Reader.create(reader);
                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.google.protobuf.Timestamp();
                while (reader.pos < end) {
                    var tag = reader.uint32();
                    switch (tag >>> 3) {
                    case 1:
                        message.seconds = reader.int64();
                        break;
                    case 2:
                        message.nanos = reader.int32();
                        break;
                    default:
                        reader.skipType(tag & 7);
                        break;
                    }
                }
                return message;
            };

            /**
             * Decodes a Timestamp message from the specified reader or buffer, length delimited.
             * @function decodeDelimited
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
             * @returns {google.protobuf.Timestamp} Timestamp
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            Timestamp.decodeDelimited = function decodeDelimited(reader) {
                if (!(reader instanceof $Reader))
                    reader = new $Reader(reader);
                return this.decode(reader, reader.uint32());
            };

            /**
             * Verifies a Timestamp message.
             * @function verify
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {Object.<string,*>} message Plain object to verify
             * @returns {string|null} `null` if valid, otherwise the reason why it is not
             */
            Timestamp.verify = function verify(message) {
                if (typeof message !== "object" || message === null)
                    return "object expected";
                if (message.seconds != null && message.hasOwnProperty("seconds"))
                    if (!$util.isInteger(message.seconds) && !(message.seconds && $util.isInteger(message.seconds.low) && $util.isInteger(message.seconds.high)))
                        return "seconds: integer|Long expected";
                if (message.nanos != null && message.hasOwnProperty("nanos"))
                    if (!$util.isInteger(message.nanos))
                        return "nanos: integer expected";
                return null;
            };

            /**
             * Creates a Timestamp message from a plain object. Also converts values to their respective internal types.
             * @function fromObject
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {Object.<string,*>} object Plain object
             * @returns {google.protobuf.Timestamp} Timestamp
             */
            Timestamp.fromObject = function fromObject(object) {
                if (object instanceof $root.google.protobuf.Timestamp)
                    return object;
                var message = new $root.google.protobuf.Timestamp();
                if (object.seconds != null)
                    if ($util.Long)
                        (message.seconds = $util.Long.fromValue(object.seconds)).unsigned = false;
                    else if (typeof object.seconds === "string")
                        message.seconds = parseInt(object.seconds, 10);
                    else if (typeof object.seconds === "number")
                        message.seconds = object.seconds;
                    else if (typeof object.seconds === "object")
                        message.seconds = new $util.LongBits(object.seconds.low >>> 0, object.seconds.high >>> 0).toNumber();
                if (object.nanos != null)
                    message.nanos = object.nanos | 0;
                return message;
            };

            /**
             * Creates a plain object from a Timestamp message. Also converts values to other types if specified.
             * @function toObject
             * @memberof google.protobuf.Timestamp
             * @static
             * @param {google.protobuf.Timestamp} message Timestamp
             * @param {$protobuf.IConversionOptions} [options] Conversion options
             * @returns {Object.<string,*>} Plain object
             */
            Timestamp.toObject = function toObject(message, options) {
                if (!options)
                    options = {};
                var object = {};
                if (options.defaults) {
                    if ($util.Long) {
                        var long = new $util.Long(0, 0, false);
                        object.seconds = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                    } else
                        object.seconds = options.longs === String ? "0" : 0;
                    object.nanos = 0;
                }
                if (message.seconds != null && message.hasOwnProperty("seconds"))
                    if (typeof message.seconds === "number")
                        object.seconds = options.longs === String ? String(message.seconds) : message.seconds;
                    else
                        object.seconds = options.longs === String ? $util.Long.prototype.toString.call(message.seconds) : options.longs === Number ? new $util.LongBits(message.seconds.low >>> 0, message.seconds.high >>> 0).toNumber() : message.seconds;
                if (message.nanos != null && message.hasOwnProperty("nanos"))
                    object.nanos = message.nanos;
                return object;
            };

            /**
             * Converts this Timestamp to JSON.
             * @function toJSON
             * @memberof google.protobuf.Timestamp
             * @instance
             * @returns {Object.<string,*>} JSON object
             */
            Timestamp.prototype.toJSON = function toJSON() {
                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
            };

            return Timestamp;
        })();

        return protobuf;
    })();

    return google;
})();

module.exports = $root;
