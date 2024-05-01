const {
  client: {
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
  },
  server: {
    jsonToProtobufHandlerWrapper,
    error: {
      wrapInErrorHandlerFactory,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BroadcastStateTransitionRequest,
    WaitForStateTransitionResultRequest,
    GetConsensusParamsRequest,
    pbjs: {
      BroadcastStateTransitionRequest: PBJSBroadcastStateTransitionRequest,
      BroadcastStateTransitionResponse: PBJSBroadcastStateTransitionResponse,
      WaitForStateTransitionResultRequest: PBJSWaitForStateTransitionResultRequest,
      WaitForStateTransitionResultResponse: PBJSWaitForStateTransitionResultResponse,
      GetConsensusParamsRequest: PBJSGetConsensusParamsRequest,
      GetConsensusParamsResponse: PBJSGetConsensusParamsResponse,
    },
  },
} = require('@dashevo/dapi-grpc');

const logger = require('../../../logger');

const createGrpcErrorFromDriveResponse = require('../createGrpcErrorFromDriveResponse');

const broadcastStateTransitionHandlerFactory = require(
  './broadcastStateTransitionHandlerFactory',
);
const waitForStateTransitionResultHandlerFactory = require(
  './waitForStateTransitionResultHandlerFactory',
);
const getConsensusParamsHandlerFactory = require(
  './getConsensusParamsHandlerFactory',
);
const unimplementedHandlerFactory = require(
  './unimplementedHandlerFactory',
);

const fetchProofForStateTransitionFactory = require('../../../externalApis/drive/fetchProofForStateTransitionFactory');
const waitForTransactionToBeProvableFactory = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');
const waitForTransactionResult = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionResult');
const getExistingTransactionResultFactory = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/getExistingTransactionResult');
const getConsensusParamsFactory = require('../../../externalApis/tenderdash/getConsensusParamsFactory');

/**
 * @param {jaysonClient} rpcClient
 * @param {BlockchainListener} blockchainListener
 * @param {PlatformPromiseClient} driveClient
 * @param {DashPlatformProtocol} dpp
 * @param {boolean} isProductionEnvironment
 * @returns {Object<string, function>}
 */
function platformHandlersFactory(
  rpcClient,
  blockchainListener,
  driveClient,
  dpp,
  isProductionEnvironment,
) {
  const wrapInErrorHandler = wrapInErrorHandlerFactory(logger, isProductionEnvironment);

  // broadcastStateTransition
  const broadcastStateTransitionHandler = broadcastStateTransitionHandlerFactory(
    rpcClient,
    createGrpcErrorFromDriveResponse,
  );

  const wrappedBroadcastStateTransition = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      BroadcastStateTransitionRequest,
      PBJSBroadcastStateTransitionRequest,
    ),
    protobufToJsonFactory(
      PBJSBroadcastStateTransitionResponse,
    ),
    wrapInErrorHandler(broadcastStateTransitionHandler),
  );

  // waitForStateTransitionResult
  const fetchProofForStateTransition = fetchProofForStateTransitionFactory(driveClient);

  const getExistingTransactionResult = getExistingTransactionResultFactory(
    rpcClient,
  );

  const waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
    waitForTransactionResult,
    getExistingTransactionResult,
  );

  const waitForStateTransitionResultHandler = waitForStateTransitionResultHandlerFactory(
    fetchProofForStateTransition,
    waitForTransactionToBeProvable,
    blockchainListener,
    dpp,
    createGrpcErrorFromDriveResponse,
  );

  const wrappedWaitForStateTransitionResult = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      WaitForStateTransitionResultRequest,
      PBJSWaitForStateTransitionResultRequest,
    ),
    protobufToJsonFactory(
      PBJSWaitForStateTransitionResultResponse,
    ),
    wrapInErrorHandler(waitForStateTransitionResultHandler),
  );

  // get Consensus Params
  const getConsensusParams = getConsensusParamsFactory(rpcClient);
  const getConsensusParamsHandler = getConsensusParamsHandlerFactory(getConsensusParams);

  const wrappedGetConsensusParams = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetConsensusParamsRequest,
      PBJSGetConsensusParamsRequest,
    ),
    protobufToJsonFactory(
      PBJSGetConsensusParamsResponse,
    ),
    wrapInErrorHandler(getConsensusParamsHandler),
  );

  return {
    broadcastStateTransition: wrappedBroadcastStateTransition,
    getIdentity: wrapInErrorHandler(unimplementedHandlerFactory('getIdentity')),
    getIdentitiesContractKeys: wrapInErrorHandler(unimplementedHandlerFactory('getIdentitiesContractKeys')),
    getIdentityBalance: wrapInErrorHandler(unimplementedHandlerFactory('getIdentityBalance')),
    getIdentityBalanceAndRevision: wrapInErrorHandler(unimplementedHandlerFactory('getIdentityBalanceAndRevision')),
    getIdentityKeys: wrapInErrorHandler(unimplementedHandlerFactory('getIdentityKeys')),
    getDocuments: wrapInErrorHandler(unimplementedHandlerFactory('getDocuments')),
    getDataContract: wrapInErrorHandler(unimplementedHandlerFactory('getDataContract')),
    getDataContracts: wrapInErrorHandler(unimplementedHandlerFactory('getDataContracts')),
    getDataContractHistory: wrapInErrorHandler(unimplementedHandlerFactory('getDataContractHistory')),
    getIdentityByPublicKeyHash: wrapInErrorHandler(unimplementedHandlerFactory('getIdentityByPublicKeyHash')),
    getIdentitiesByPublicKeyHashes: wrapInErrorHandler(unimplementedHandlerFactory('getIdentitiesByPublicKeyHashes')),
    waitForStateTransitionResult: wrappedWaitForStateTransitionResult,
    getConsensusParams: wrappedGetConsensusParams,
    getProofs: wrapInErrorHandler(unimplementedHandlerFactory('getProofs')),
    getEpochsInfo: wrapInErrorHandler(unimplementedHandlerFactory('getEpochsInfo')),
    getProtocolVersionUpgradeVoteStatus: wrapInErrorHandler(unimplementedHandlerFactory('getProtocolVersionUpgradeVoteStatus')),
    getProtocolVersionUpgradeState: wrapInErrorHandler(unimplementedHandlerFactory('getProtocolVersionUpgradeState')),
    getIdentityContractNonce: wrapInErrorHandler(unimplementedHandlerFactory('getIdentityContractNonce')),
    getIdentityNonce: wrapInErrorHandler(unimplementedHandlerFactory('getIdentityNonce')),
  };
}

module.exports = platformHandlersFactory;
