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
    GetIdentityRequest,
    GetDataContractRequest,
    GetDataContractHistoryRequest,
    GetDocumentsRequest,
    GetIdentitiesByPublicKeyHashesRequest,
    WaitForStateTransitionResultRequest,
    GetConsensusParamsRequest,
    GetEpochsInfoRequest,
    GetProtocolVersionUpgradeVoteStatusRequest,
    GetProtocolVersionUpgradeStateRequest,
    pbjs: {
      BroadcastStateTransitionRequest: PBJSBroadcastStateTransitionRequest,
      BroadcastStateTransitionResponse: PBJSBroadcastStateTransitionResponse,
      GetIdentityRequest: PBJSGetIdentityRequest,
      GetIdentityResponse: PBJSGetIdentityResponse,
      GetDataContractRequest: PBJSGetDataContractRequest,
      GetDataContractResponse: PBJSGetDataContractResponse,
      GetDocumentsRequest: PBJSGetDocumentsRequest,
      GetDocumentsResponse: PBJSGetDocumentsResponse,
      GetIdentitiesByPublicKeyHashesResponse: PBJSGetIdentitiesByPublicKeyHashesResponse,
      GetIdentitiesByPublicKeyHashesRequest: PBJSGetIdentitiesByPublicKeyHashesRequest,
      WaitForStateTransitionResultRequest: PBJSWaitForStateTransitionResultRequest,
      WaitForStateTransitionResultResponse: PBJSWaitForStateTransitionResultResponse,
      GetConsensusParamsRequest: PBJSGetConsensusParamsRequest,
      GetConsensusParamsResponse: PBJSGetConsensusParamsResponse,
      GetDataContractHistoryRequest: PBJSGetDataContractHistoryRequest,
      GetDataContractHistoryResponse: PBJSGetDataContractHistoryResponse,
      GetEpochsInfoRequest: PBJSGetEpochsInfoRequest,
      GetEpochsInfoResponse: PBJSGetEpochsInfoResponse,
      GetProtocolVersionUpgradeVoteStatusRequest: PBJSGetProtocolVersionUpgradeVoteStatusRequest,
      GetProtocolVersionUpgradeVoteStatusResponse: PBJSGetProtocolVersionUpgradeVoteStatusResponse,
      GetProtocolVersionUpgradeStateRequest: PBJSGetProtocolVersionUpgradeStateRequest,
      GetProtocolVersionUpgradeStateResponse: PBJSGetProtocolVersionUpgradeStateResponse,
    },
  },
} = require('@dashevo/dapi-grpc');

const log = require('../../../log');

const createGrpcErrorFromDriveResponse = require('../createGrpcErrorFromDriveResponse');

const getIdentityHandlerFactory = require(
  './getIdentityHandlerFactory',
);
const broadcastStateTransitionHandlerFactory = require(
  './broadcastStateTransitionHandlerFactory',
);
const getDocumentsHandlerFactory = require(
  './getDocumentsHandlerFactory',
);
const getDataContractHandlerFactory = require(
  './getDataContractHandlerFactory',
);
const getDataContractHistoryHandlerFactory = require(
  './getDataContractHistoryHandlerFactory',
);
const getIdentitiesByPublicKeyHashesHandlerFactory = require(
  './getIdentitiesByPublicKeyHashesHandlerFactory',
);
const waitForStateTransitionResultHandlerFactory = require(
  './waitForStateTransitionResultHandlerFactory',
);
const getConsensusParamsHandlerFactory = require(
  './getConsensusParamsHandlerFactory',
);
const getEpochsInfoHandlerFactory = require(
  './getEpochsInfoHandlerFactory',
);

const getProtocolVersionUpgradeVoteStatusHandlerFactory = require(
  './getProtocolVersionUpgradeVoteStatusHandlerFactory',
);

const getProtocolVersionUpgradeStateHandlerFactory = require(
  './getProtocolVersionUpgradeStateHandlerFactory',
);

const fetchProofForStateTransitionFactory = require('../../../externalApis/drive/fetchProofForStateTransitionFactory');
const waitForTransactionToBeProvableFactory = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');
const waitForTransactionResult = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionResult');
const getExistingTransactionResultFactory = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/getExistingTransactionResult');
const getConsensusParamsFactory = require('../../../externalApis/tenderdash/getConsensusParamsFactory');

/**
 * @param {jaysonClient} rpcClient
 * @param {BlockchainListener} blockchainListener
 * @param {DriveClient} driveClient
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
  const wrapInErrorHandler = wrapInErrorHandlerFactory(log, isProductionEnvironment);

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

  // getIdentity
  const getIdentityHandler = getIdentityHandlerFactory(
    driveClient,
  );

  const wrappedGetIdentity = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityRequest,
      PBJSGetIdentityRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityResponse,
    ),
    wrapInErrorHandler(getIdentityHandler),
  );

  // getDocuments
  const getDocumentsHandler = getDocumentsHandlerFactory(
    driveClient,
  );

  const wrappedGetDocuments = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetDocumentsRequest,
      PBJSGetDocumentsRequest,
    ),
    protobufToJsonFactory(
      PBJSGetDocumentsResponse,
    ),
    wrapInErrorHandler(getDocumentsHandler),
  );

  // getDataContract
  const getDataContractHandler = getDataContractHandlerFactory(
    driveClient,
  );

  const wrappedGetDataContract = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetDataContractRequest,
      PBJSGetDataContractRequest,
    ),
    protobufToJsonFactory(
      PBJSGetDataContractResponse,
    ),
    wrapInErrorHandler(getDataContractHandler),
  );

  // getDataContractHistory
  const getDataContractHistoryHandler = getDataContractHistoryHandlerFactory(
    driveClient,
  );

  const wrappedGetDataContractHistory = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetDataContractHistoryRequest,
      PBJSGetDataContractHistoryRequest,
    ),
    protobufToJsonFactory(
      PBJSGetDataContractHistoryResponse,
    ),
    wrapInErrorHandler(getDataContractHistoryHandler),
  );

  // getIdentitiesByPublicKeyHashes
  const getIdentitiesByPublicKeyHashesHandler = getIdentitiesByPublicKeyHashesHandlerFactory(
    driveClient,
  );

  const wrappedGetIdentitiesByPublicKeyHashes = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentitiesByPublicKeyHashesRequest,
      PBJSGetIdentitiesByPublicKeyHashesRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentitiesByPublicKeyHashesResponse,
    ),
    wrapInErrorHandler(getIdentitiesByPublicKeyHashesHandler),
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

  // getEpochsInfo
  const getEpochsInfoHandler = getEpochsInfoHandlerFactory(
    driveClient,
  );

  const wrappedGetEpochsInfo = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetEpochsInfoRequest,
      PBJSGetEpochsInfoRequest,
    ),
    protobufToJsonFactory(
      PBJSGetEpochsInfoResponse,
    ),
    wrapInErrorHandler(getEpochsInfoHandler),
  );

  // getProtocolVersionUpgradeVoteStatus
  const getProtocolVersionUpgradeVoteStatusHandler = getProtocolVersionUpgradeVoteStatusHandlerFactory(
    driveClient,
  );

  const wrappedGetProtocolVersionUpgradeVoteStatus = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetProtocolVersionUpgradeVoteStatusRequest,
      PBJSGetProtocolVersionUpgradeVoteStatusRequest,
    ),
    protobufToJsonFactory(
      PBJSGetProtocolVersionUpgradeVoteStatusResponse,
    ),
    wrapInErrorHandler(getProtocolVersionUpgradeVoteStatusHandler),
  );

  // getProtocolVersionUpgradeState
  const getProtocolVersionUpgradeStateHandler = getProtocolVersionUpgradeStateHandlerFactory(
    driveClient,
  );

  const wrappedGetProtocolVersionUpgradeState = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetProtocolVersionUpgradeStateRequest,
      PBJSGetProtocolVersionUpgradeStateRequest,
    ),
    protobufToJsonFactory(
      PBJSGetProtocolVersionUpgradeStateResponse,
    ),
    wrapInErrorHandler(getProtocolVersionUpgradeStateHandler),
  );

  return {
    broadcastStateTransition: wrappedBroadcastStateTransition,
    getIdentity: wrappedGetIdentity,
    getDocuments: wrappedGetDocuments,
    getDataContract: wrappedGetDataContract,
    getDataContractHistory: wrappedGetDataContractHistory,
    getIdentitiesByPublicKeyHashes: wrappedGetIdentitiesByPublicKeyHashes,
    waitForStateTransitionResult: wrappedWaitForStateTransitionResult,
    getConsensusParams: wrappedGetConsensusParams,
    getEpochsInfo: wrappedGetEpochsInfo,
    getProtocolVersionUpgradeVoteStatus: wrappedGetProtocolVersionUpgradeVoteStatus,
    getProtocolVersionUpgradeState: wrappedGetProtocolVersionUpgradeState,
  };
}

module.exports = platformHandlersFactory;
