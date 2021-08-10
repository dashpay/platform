/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars*/
"use strict";

var $protobuf = require("protobufjs/minimal");

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
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentityIdsByPublicKeyHashes}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityIdsByPublicKeyHashesCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse} [response] GetIdentityIdsByPublicKeyHashesResponse
                         */

                        /**
                         * Calls getIdentityIdsByPublicKeyHashes.
                         * @function getIdentityIdsByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesRequest} request GetIdentityIdsByPublicKeyHashesRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityIdsByPublicKeyHashesCallback} callback Node-style callback called with the error, if any, and GetIdentityIdsByPublicKeyHashesResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentityIdsByPublicKeyHashes = function getIdentityIdsByPublicKeyHashes(request, callback) {
                            return this.rpcCall(getIdentityIdsByPublicKeyHashes, $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest, $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse, request, callback);
                        }, "name", { value: "getIdentityIdsByPublicKeyHashes" });

                        /**
                         * Calls getIdentityIdsByPublicKeyHashes.
                         * @function getIdentityIdsByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesRequest} request GetIdentityIdsByPublicKeyHashesRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse>} Promise
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

                    v0.StoreTreeProofs = (function() {

                        /**
                         * Properties of a StoreTreeProofs.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IStoreTreeProofs
                         * @property {Uint8Array|null} [identitiesProof] StoreTreeProofs identitiesProof
                         * @property {Uint8Array|null} [publicKeyHashesToIdentityIdsProof] StoreTreeProofs publicKeyHashesToIdentityIdsProof
                         * @property {Uint8Array|null} [dataContractsProof] StoreTreeProofs dataContractsProof
                         * @property {Uint8Array|null} [documentsProof] StoreTreeProofs documentsProof
                         */

                        /**
                         * Constructs a new StoreTreeProofs.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a StoreTreeProofs.
                         * @implements IStoreTreeProofs
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IStoreTreeProofs=} [properties] Properties to set
                         */
                        function StoreTreeProofs(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * StoreTreeProofs identitiesProof.
                         * @member {Uint8Array} identitiesProof
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @instance
                         */
                        StoreTreeProofs.prototype.identitiesProof = $util.newBuffer([]);

                        /**
                         * StoreTreeProofs publicKeyHashesToIdentityIdsProof.
                         * @member {Uint8Array} publicKeyHashesToIdentityIdsProof
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @instance
                         */
                        StoreTreeProofs.prototype.publicKeyHashesToIdentityIdsProof = $util.newBuffer([]);

                        /**
                         * StoreTreeProofs dataContractsProof.
                         * @member {Uint8Array} dataContractsProof
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @instance
                         */
                        StoreTreeProofs.prototype.dataContractsProof = $util.newBuffer([]);

                        /**
                         * StoreTreeProofs documentsProof.
                         * @member {Uint8Array} documentsProof
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @instance
                         */
                        StoreTreeProofs.prototype.documentsProof = $util.newBuffer([]);

                        /**
                         * Creates a new StoreTreeProofs instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStoreTreeProofs=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.StoreTreeProofs} StoreTreeProofs instance
                         */
                        StoreTreeProofs.create = function create(properties) {
                            return new StoreTreeProofs(properties);
                        };

                        /**
                         * Encodes the specified StoreTreeProofs message. Does not implicitly {@link org.dash.platform.dapi.v0.StoreTreeProofs.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStoreTreeProofs} message StoreTreeProofs message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        StoreTreeProofs.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.identitiesProof != null && Object.hasOwnProperty.call(message, "identitiesProof"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.identitiesProof);
                            if (message.publicKeyHashesToIdentityIdsProof != null && Object.hasOwnProperty.call(message, "publicKeyHashesToIdentityIdsProof"))
                                writer.uint32(/* id 2, wireType 2 =*/18).bytes(message.publicKeyHashesToIdentityIdsProof);
                            if (message.dataContractsProof != null && Object.hasOwnProperty.call(message, "dataContractsProof"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.dataContractsProof);
                            if (message.documentsProof != null && Object.hasOwnProperty.call(message, "documentsProof"))
                                writer.uint32(/* id 4, wireType 2 =*/34).bytes(message.documentsProof);
                            return writer;
                        };

                        /**
                         * Encodes the specified StoreTreeProofs message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.StoreTreeProofs.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStoreTreeProofs} message StoreTreeProofs message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        StoreTreeProofs.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a StoreTreeProofs message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.StoreTreeProofs} StoreTreeProofs
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        StoreTreeProofs.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.StoreTreeProofs();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.identitiesProof = reader.bytes();
                                    break;
                                case 2:
                                    message.publicKeyHashesToIdentityIdsProof = reader.bytes();
                                    break;
                                case 3:
                                    message.dataContractsProof = reader.bytes();
                                    break;
                                case 4:
                                    message.documentsProof = reader.bytes();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a StoreTreeProofs message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.StoreTreeProofs} StoreTreeProofs
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        StoreTreeProofs.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a StoreTreeProofs message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        StoreTreeProofs.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.identitiesProof != null && message.hasOwnProperty("identitiesProof"))
                                if (!(message.identitiesProof && typeof message.identitiesProof.length === "number" || $util.isString(message.identitiesProof)))
                                    return "identitiesProof: buffer expected";
                            if (message.publicKeyHashesToIdentityIdsProof != null && message.hasOwnProperty("publicKeyHashesToIdentityIdsProof"))
                                if (!(message.publicKeyHashesToIdentityIdsProof && typeof message.publicKeyHashesToIdentityIdsProof.length === "number" || $util.isString(message.publicKeyHashesToIdentityIdsProof)))
                                    return "publicKeyHashesToIdentityIdsProof: buffer expected";
                            if (message.dataContractsProof != null && message.hasOwnProperty("dataContractsProof"))
                                if (!(message.dataContractsProof && typeof message.dataContractsProof.length === "number" || $util.isString(message.dataContractsProof)))
                                    return "dataContractsProof: buffer expected";
                            if (message.documentsProof != null && message.hasOwnProperty("documentsProof"))
                                if (!(message.documentsProof && typeof message.documentsProof.length === "number" || $util.isString(message.documentsProof)))
                                    return "documentsProof: buffer expected";
                            return null;
                        };

                        /**
                         * Creates a StoreTreeProofs message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.StoreTreeProofs} StoreTreeProofs
                         */
                        StoreTreeProofs.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.StoreTreeProofs)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.StoreTreeProofs();
                            if (object.identitiesProof != null)
                                if (typeof object.identitiesProof === "string")
                                    $util.base64.decode(object.identitiesProof, message.identitiesProof = $util.newBuffer($util.base64.length(object.identitiesProof)), 0);
                                else if (object.identitiesProof.length >= 0)
                                    message.identitiesProof = object.identitiesProof;
                            if (object.publicKeyHashesToIdentityIdsProof != null)
                                if (typeof object.publicKeyHashesToIdentityIdsProof === "string")
                                    $util.base64.decode(object.publicKeyHashesToIdentityIdsProof, message.publicKeyHashesToIdentityIdsProof = $util.newBuffer($util.base64.length(object.publicKeyHashesToIdentityIdsProof)), 0);
                                else if (object.publicKeyHashesToIdentityIdsProof.length >= 0)
                                    message.publicKeyHashesToIdentityIdsProof = object.publicKeyHashesToIdentityIdsProof;
                            if (object.dataContractsProof != null)
                                if (typeof object.dataContractsProof === "string")
                                    $util.base64.decode(object.dataContractsProof, message.dataContractsProof = $util.newBuffer($util.base64.length(object.dataContractsProof)), 0);
                                else if (object.dataContractsProof.length >= 0)
                                    message.dataContractsProof = object.dataContractsProof;
                            if (object.documentsProof != null)
                                if (typeof object.documentsProof === "string")
                                    $util.base64.decode(object.documentsProof, message.documentsProof = $util.newBuffer($util.base64.length(object.documentsProof)), 0);
                                else if (object.documentsProof.length >= 0)
                                    message.documentsProof = object.documentsProof;
                            return message;
                        };

                        /**
                         * Creates a plain object from a StoreTreeProofs message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @static
                         * @param {org.dash.platform.dapi.v0.StoreTreeProofs} message StoreTreeProofs
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        StoreTreeProofs.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.identitiesProof = "";
                                else {
                                    object.identitiesProof = [];
                                    if (options.bytes !== Array)
                                        object.identitiesProof = $util.newBuffer(object.identitiesProof);
                                }
                                if (options.bytes === String)
                                    object.publicKeyHashesToIdentityIdsProof = "";
                                else {
                                    object.publicKeyHashesToIdentityIdsProof = [];
                                    if (options.bytes !== Array)
                                        object.publicKeyHashesToIdentityIdsProof = $util.newBuffer(object.publicKeyHashesToIdentityIdsProof);
                                }
                                if (options.bytes === String)
                                    object.dataContractsProof = "";
                                else {
                                    object.dataContractsProof = [];
                                    if (options.bytes !== Array)
                                        object.dataContractsProof = $util.newBuffer(object.dataContractsProof);
                                }
                                if (options.bytes === String)
                                    object.documentsProof = "";
                                else {
                                    object.documentsProof = [];
                                    if (options.bytes !== Array)
                                        object.documentsProof = $util.newBuffer(object.documentsProof);
                                }
                            }
                            if (message.identitiesProof != null && message.hasOwnProperty("identitiesProof"))
                                object.identitiesProof = options.bytes === String ? $util.base64.encode(message.identitiesProof, 0, message.identitiesProof.length) : options.bytes === Array ? Array.prototype.slice.call(message.identitiesProof) : message.identitiesProof;
                            if (message.publicKeyHashesToIdentityIdsProof != null && message.hasOwnProperty("publicKeyHashesToIdentityIdsProof"))
                                object.publicKeyHashesToIdentityIdsProof = options.bytes === String ? $util.base64.encode(message.publicKeyHashesToIdentityIdsProof, 0, message.publicKeyHashesToIdentityIdsProof.length) : options.bytes === Array ? Array.prototype.slice.call(message.publicKeyHashesToIdentityIdsProof) : message.publicKeyHashesToIdentityIdsProof;
                            if (message.dataContractsProof != null && message.hasOwnProperty("dataContractsProof"))
                                object.dataContractsProof = options.bytes === String ? $util.base64.encode(message.dataContractsProof, 0, message.dataContractsProof.length) : options.bytes === Array ? Array.prototype.slice.call(message.dataContractsProof) : message.dataContractsProof;
                            if (message.documentsProof != null && message.hasOwnProperty("documentsProof"))
                                object.documentsProof = options.bytes === String ? $util.base64.encode(message.documentsProof, 0, message.documentsProof.length) : options.bytes === Array ? Array.prototype.slice.call(message.documentsProof) : message.documentsProof;
                            return object;
                        };

                        /**
                         * Converts this StoreTreeProofs to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.StoreTreeProofs
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        StoreTreeProofs.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return StoreTreeProofs;
                    })();

                    v0.Proof = (function() {

                        /**
                         * Properties of a Proof.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IProof
                         * @property {Uint8Array|null} [rootTreeProof] Proof rootTreeProof
                         * @property {org.dash.platform.dapi.v0.IStoreTreeProofs|null} [storeTreeProofs] Proof storeTreeProofs
                         * @property {Uint8Array|null} [signatureLlmqHash] Proof signatureLlmqHash
                         * @property {Uint8Array|null} [signature] Proof signature
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
                         * Proof rootTreeProof.
                         * @member {Uint8Array} rootTreeProof
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.rootTreeProof = $util.newBuffer([]);

                        /**
                         * Proof storeTreeProofs.
                         * @member {org.dash.platform.dapi.v0.IStoreTreeProofs|null|undefined} storeTreeProofs
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.storeTreeProofs = null;

                        /**
                         * Proof signatureLlmqHash.
                         * @member {Uint8Array} signatureLlmqHash
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.signatureLlmqHash = $util.newBuffer([]);

                        /**
                         * Proof signature.
                         * @member {Uint8Array} signature
                         * @memberof org.dash.platform.dapi.v0.Proof
                         * @instance
                         */
                        Proof.prototype.signature = $util.newBuffer([]);

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
                            if (message.rootTreeProof != null && Object.hasOwnProperty.call(message, "rootTreeProof"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.rootTreeProof);
                            if (message.storeTreeProofs != null && Object.hasOwnProperty.call(message, "storeTreeProofs"))
                                $root.org.dash.platform.dapi.v0.StoreTreeProofs.encode(message.storeTreeProofs, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.signatureLlmqHash != null && Object.hasOwnProperty.call(message, "signatureLlmqHash"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.signatureLlmqHash);
                            if (message.signature != null && Object.hasOwnProperty.call(message, "signature"))
                                writer.uint32(/* id 4, wireType 2 =*/34).bytes(message.signature);
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
                                    message.rootTreeProof = reader.bytes();
                                    break;
                                case 2:
                                    message.storeTreeProofs = $root.org.dash.platform.dapi.v0.StoreTreeProofs.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.signatureLlmqHash = reader.bytes();
                                    break;
                                case 4:
                                    message.signature = reader.bytes();
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
                            if (message.rootTreeProof != null && message.hasOwnProperty("rootTreeProof"))
                                if (!(message.rootTreeProof && typeof message.rootTreeProof.length === "number" || $util.isString(message.rootTreeProof)))
                                    return "rootTreeProof: buffer expected";
                            if (message.storeTreeProofs != null && message.hasOwnProperty("storeTreeProofs")) {
                                var error = $root.org.dash.platform.dapi.v0.StoreTreeProofs.verify(message.storeTreeProofs);
                                if (error)
                                    return "storeTreeProofs." + error;
                            }
                            if (message.signatureLlmqHash != null && message.hasOwnProperty("signatureLlmqHash"))
                                if (!(message.signatureLlmqHash && typeof message.signatureLlmqHash.length === "number" || $util.isString(message.signatureLlmqHash)))
                                    return "signatureLlmqHash: buffer expected";
                            if (message.signature != null && message.hasOwnProperty("signature"))
                                if (!(message.signature && typeof message.signature.length === "number" || $util.isString(message.signature)))
                                    return "signature: buffer expected";
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
                            if (object.rootTreeProof != null)
                                if (typeof object.rootTreeProof === "string")
                                    $util.base64.decode(object.rootTreeProof, message.rootTreeProof = $util.newBuffer($util.base64.length(object.rootTreeProof)), 0);
                                else if (object.rootTreeProof.length >= 0)
                                    message.rootTreeProof = object.rootTreeProof;
                            if (object.storeTreeProofs != null) {
                                if (typeof object.storeTreeProofs !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.Proof.storeTreeProofs: object expected");
                                message.storeTreeProofs = $root.org.dash.platform.dapi.v0.StoreTreeProofs.fromObject(object.storeTreeProofs);
                            }
                            if (object.signatureLlmqHash != null)
                                if (typeof object.signatureLlmqHash === "string")
                                    $util.base64.decode(object.signatureLlmqHash, message.signatureLlmqHash = $util.newBuffer($util.base64.length(object.signatureLlmqHash)), 0);
                                else if (object.signatureLlmqHash.length >= 0)
                                    message.signatureLlmqHash = object.signatureLlmqHash;
                            if (object.signature != null)
                                if (typeof object.signature === "string")
                                    $util.base64.decode(object.signature, message.signature = $util.newBuffer($util.base64.length(object.signature)), 0);
                                else if (object.signature.length >= 0)
                                    message.signature = object.signature;
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
                                    object.rootTreeProof = "";
                                else {
                                    object.rootTreeProof = [];
                                    if (options.bytes !== Array)
                                        object.rootTreeProof = $util.newBuffer(object.rootTreeProof);
                                }
                                object.storeTreeProofs = null;
                                if (options.bytes === String)
                                    object.signatureLlmqHash = "";
                                else {
                                    object.signatureLlmqHash = [];
                                    if (options.bytes !== Array)
                                        object.signatureLlmqHash = $util.newBuffer(object.signatureLlmqHash);
                                }
                                if (options.bytes === String)
                                    object.signature = "";
                                else {
                                    object.signature = [];
                                    if (options.bytes !== Array)
                                        object.signature = $util.newBuffer(object.signature);
                                }
                            }
                            if (message.rootTreeProof != null && message.hasOwnProperty("rootTreeProof"))
                                object.rootTreeProof = options.bytes === String ? $util.base64.encode(message.rootTreeProof, 0, message.rootTreeProof.length) : options.bytes === Array ? Array.prototype.slice.call(message.rootTreeProof) : message.rootTreeProof;
                            if (message.storeTreeProofs != null && message.hasOwnProperty("storeTreeProofs"))
                                object.storeTreeProofs = $root.org.dash.platform.dapi.v0.StoreTreeProofs.toObject(message.storeTreeProofs, options);
                            if (message.signatureLlmqHash != null && message.hasOwnProperty("signatureLlmqHash"))
                                object.signatureLlmqHash = options.bytes === String ? $util.base64.encode(message.signatureLlmqHash, 0, message.signatureLlmqHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.signatureLlmqHash) : message.signatureLlmqHash;
                            if (message.signature != null && message.hasOwnProperty("signature"))
                                object.signature = options.bytes === String ? $util.base64.encode(message.signature, 0, message.signature.length) : options.bytes === Array ? Array.prototype.slice.call(message.signature) : message.signature;
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
                        ResponseMetadata.prototype.height = $util.Long ? $util.Long.fromBits(0,0,false) : 0;

                        /**
                         * ResponseMetadata coreChainLockedHeight.
                         * @member {number} coreChainLockedHeight
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.coreChainLockedHeight = 0;

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
                                writer.uint32(/* id 1, wireType 0 =*/8).int64(message.height);
                            if (message.coreChainLockedHeight != null && Object.hasOwnProperty.call(message, "coreChainLockedHeight"))
                                writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.coreChainLockedHeight);
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
                                    message.height = reader.int64();
                                    break;
                                case 2:
                                    message.coreChainLockedHeight = reader.uint32();
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
                                    (message.height = $util.Long.fromValue(object.height)).unsigned = false;
                                else if (typeof object.height === "string")
                                    message.height = parseInt(object.height, 10);
                                else if (typeof object.height === "number")
                                    message.height = object.height;
                                else if (typeof object.height === "object")
                                    message.height = new $util.LongBits(object.height.low >>> 0, object.height.high >>> 0).toNumber();
                            if (object.coreChainLockedHeight != null)
                                message.coreChainLockedHeight = object.coreChainLockedHeight >>> 0;
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
                                    var long = new $util.Long(0, 0, false);
                                    object.height = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                                } else
                                    object.height = options.longs === String ? "0" : 0;
                                object.coreChainLockedHeight = 0;
                            }
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (typeof message.height === "number")
                                    object.height = options.longs === String ? String(message.height) : message.height;
                                else
                                    object.height = options.longs === String ? $util.Long.prototype.toString.call(message.height) : options.longs === Number ? new $util.LongBits(message.height.low >>> 0, message.height.high >>> 0).toNumber() : message.height;
                            if (message.coreChainLockedHeight != null && message.hasOwnProperty("coreChainLockedHeight"))
                                object.coreChainLockedHeight = message.coreChainLockedHeight;
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
                         * @property {number|null} [startAfter] GetDocumentsRequest startAfter
                         * @property {number|null} [startAt] GetDocumentsRequest startAt
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
                         * @member {number} startAfter
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.startAfter = 0;

                        /**
                         * GetDocumentsRequest startAt.
                         * @member {number} startAt
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.startAt = 0;

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
                                writer.uint32(/* id 6, wireType 0 =*/48).uint32(message.startAfter);
                            if (message.startAt != null && Object.hasOwnProperty.call(message, "startAt"))
                                writer.uint32(/* id 7, wireType 0 =*/56).uint32(message.startAt);
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
                                    message.startAfter = reader.uint32();
                                    break;
                                case 7:
                                    message.startAt = reader.uint32();
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
                                if (!$util.isInteger(message.startAfter))
                                    return "startAfter: integer expected";
                            }
                            if (message.startAt != null && message.hasOwnProperty("startAt")) {
                                if (properties.start === 1)
                                    return "start: multiple values";
                                properties.start = 1;
                                if (!$util.isInteger(message.startAt))
                                    return "startAt: integer expected";
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
                                message.startAfter = object.startAfter >>> 0;
                            if (object.startAt != null)
                                message.startAt = object.startAt >>> 0;
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
                                object.startAfter = message.startAfter;
                                if (options.oneofs)
                                    object.start = "startAfter";
                            }
                            if (message.startAt != null && message.hasOwnProperty("startAt")) {
                                object.startAt = message.startAt;
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

                    v0.GetIdentityIdsByPublicKeyHashesRequest = (function() {

                        /**
                         * Properties of a GetIdentityIdsByPublicKeyHashesRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityIdsByPublicKeyHashesRequest
                         * @property {Array.<Uint8Array>|null} [publicKeyHashes] GetIdentityIdsByPublicKeyHashesRequest publicKeyHashes
                         * @property {boolean|null} [prove] GetIdentityIdsByPublicKeyHashesRequest prove
                         */

                        /**
                         * Constructs a new GetIdentityIdsByPublicKeyHashesRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityIdsByPublicKeyHashesRequest.
                         * @implements IGetIdentityIdsByPublicKeyHashesRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesRequest=} [properties] Properties to set
                         */
                        function GetIdentityIdsByPublicKeyHashesRequest(properties) {
                            this.publicKeyHashes = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityIdsByPublicKeyHashesRequest publicKeyHashes.
                         * @member {Array.<Uint8Array>} publicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @instance
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.prototype.publicKeyHashes = $util.emptyArray;

                        /**
                         * GetIdentityIdsByPublicKeyHashesRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @instance
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.prototype.prove = false;

                        /**
                         * Creates a new GetIdentityIdsByPublicKeyHashesRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} GetIdentityIdsByPublicKeyHashesRequest instance
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.create = function create(properties) {
                            return new GetIdentityIdsByPublicKeyHashesRequest(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityIdsByPublicKeyHashesRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesRequest} message GetIdentityIdsByPublicKeyHashesRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.encode = function encode(message, writer) {
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
                         * Encodes the specified GetIdentityIdsByPublicKeyHashesRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesRequest} message GetIdentityIdsByPublicKeyHashesRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityIdsByPublicKeyHashesRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} GetIdentityIdsByPublicKeyHashesRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest();
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
                         * Decodes a GetIdentityIdsByPublicKeyHashesRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} GetIdentityIdsByPublicKeyHashesRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityIdsByPublicKeyHashesRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.verify = function verify(message) {
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
                         * Creates a GetIdentityIdsByPublicKeyHashesRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} GetIdentityIdsByPublicKeyHashesRequest
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest();
                            if (object.publicKeyHashes) {
                                if (!Array.isArray(object.publicKeyHashes))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest.publicKeyHashes: array expected");
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
                         * Creates a plain object from a GetIdentityIdsByPublicKeyHashesRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest} message GetIdentityIdsByPublicKeyHashesRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.toObject = function toObject(message, options) {
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
                         * Converts this GetIdentityIdsByPublicKeyHashesRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityIdsByPublicKeyHashesRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityIdsByPublicKeyHashesRequest;
                    })();

                    v0.GetIdentityIdsByPublicKeyHashesResponse = (function() {

                        /**
                         * Properties of a GetIdentityIdsByPublicKeyHashesResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetIdentityIdsByPublicKeyHashesResponse
                         * @property {Array.<Uint8Array>|null} [identityIds] GetIdentityIdsByPublicKeyHashesResponse identityIds
                         * @property {org.dash.platform.dapi.v0.IProof|null} [proof] GetIdentityIdsByPublicKeyHashesResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] GetIdentityIdsByPublicKeyHashesResponse metadata
                         */

                        /**
                         * Constructs a new GetIdentityIdsByPublicKeyHashesResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetIdentityIdsByPublicKeyHashesResponse.
                         * @implements IGetIdentityIdsByPublicKeyHashesResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesResponse=} [properties] Properties to set
                         */
                        function GetIdentityIdsByPublicKeyHashesResponse(properties) {
                            this.identityIds = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetIdentityIdsByPublicKeyHashesResponse identityIds.
                         * @member {Array.<Uint8Array>} identityIds
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.prototype.identityIds = $util.emptyArray;

                        /**
                         * GetIdentityIdsByPublicKeyHashesResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProof|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.prototype.proof = null;

                        /**
                         * GetIdentityIdsByPublicKeyHashesResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @instance
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.prototype.metadata = null;

                        /**
                         * Creates a new GetIdentityIdsByPublicKeyHashesResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse} GetIdentityIdsByPublicKeyHashesResponse instance
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.create = function create(properties) {
                            return new GetIdentityIdsByPublicKeyHashesResponse(properties);
                        };

                        /**
                         * Encodes the specified GetIdentityIdsByPublicKeyHashesResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesResponse} message GetIdentityIdsByPublicKeyHashesResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.identityIds != null && message.identityIds.length)
                                for (var i = 0; i < message.identityIds.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.identityIds[i]);
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.Proof.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetIdentityIdsByPublicKeyHashesResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetIdentityIdsByPublicKeyHashesResponse} message GetIdentityIdsByPublicKeyHashesResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetIdentityIdsByPublicKeyHashesResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse} GetIdentityIdsByPublicKeyHashesResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.identityIds && message.identityIds.length))
                                        message.identityIds = [];
                                    message.identityIds.push(reader.bytes());
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
                         * Decodes a GetIdentityIdsByPublicKeyHashesResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse} GetIdentityIdsByPublicKeyHashesResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetIdentityIdsByPublicKeyHashesResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.identityIds != null && message.hasOwnProperty("identityIds")) {
                                if (!Array.isArray(message.identityIds))
                                    return "identityIds: array expected";
                                for (var i = 0; i < message.identityIds.length; ++i)
                                    if (!(message.identityIds[i] && typeof message.identityIds[i].length === "number" || $util.isString(message.identityIds[i])))
                                        return "identityIds: buffer[] expected";
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
                         * Creates a GetIdentityIdsByPublicKeyHashesResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse} GetIdentityIdsByPublicKeyHashesResponse
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse();
                            if (object.identityIds) {
                                if (!Array.isArray(object.identityIds))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse.identityIds: array expected");
                                message.identityIds = [];
                                for (var i = 0; i < object.identityIds.length; ++i)
                                    if (typeof object.identityIds[i] === "string")
                                        $util.base64.decode(object.identityIds[i], message.identityIds[i] = $util.newBuffer($util.base64.length(object.identityIds[i])), 0);
                                    else if (object.identityIds[i].length >= 0)
                                        message.identityIds[i] = object.identityIds[i];
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.Proof.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetIdentityIdsByPublicKeyHashesResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse} message GetIdentityIdsByPublicKeyHashesResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.identityIds = [];
                            if (options.defaults) {
                                object.proof = null;
                                object.metadata = null;
                            }
                            if (message.identityIds && message.identityIds.length) {
                                object.identityIds = [];
                                for (var j = 0; j < message.identityIds.length; ++j)
                                    object.identityIds[j] = options.bytes === String ? $util.base64.encode(message.identityIds[j], 0, message.identityIds[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.identityIds[j]) : message.identityIds[j];
                            }
                            if (message.proof != null && message.hasOwnProperty("proof"))
                                object.proof = $root.org.dash.platform.dapi.v0.Proof.toObject(message.proof, options);
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this GetIdentityIdsByPublicKeyHashesResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetIdentityIdsByPublicKeyHashesResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetIdentityIdsByPublicKeyHashesResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetIdentityIdsByPublicKeyHashesResponse;
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

module.exports = $root;
