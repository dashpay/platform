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
    GetIdentityKeysRequest,
    GetDataContractRequest,
    GetDataContractsRequest,
    GetDataContractHistoryRequest,
    GetDocumentsRequest,
    GetIdentitiesRequest,
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentityByPublicKeyHashesRequest,
    WaitForStateTransitionResultRequest,
    GetConsensusParamsRequest,
    GetProofsRequest,
    GetEpochsInfoRequest,
    GetVersionUpgradeVoteStatusRequest,
    GetVersionUpgradeStateRequest,
    pbjs: {
      BroadcastStateTransitionRequest: PBJSBroadcastStateTransitionRequest,
      BroadcastStateTransitionResponse: PBJSBroadcastStateTransitionResponse,
      GetIdentityRequest: PBJSGetIdentityRequest,
      GetIdentityResponse: PBJSGetIdentityResponse,
      GetIdentitiesRequest: PBJSGetIdentitiesRequest,
      GetIdentitiesResponse: PBJSGetIdentitiesResponse,
      GetIdentityBalanceResponse: PBJSGetIdentityBalanceResponse,
      GetIdentityBalanceAndRevisionResponse: PBJSGetIdentityBalanceAndRevisionResponse,
      GetIdentityKeysRequest: PBJSGetIdentityKeysRequest,
      GetIdentityKeysResponse: PBJSGetIdentityKeysResponse,
      GetDataContractRequest: PBJSGetDataContractRequest,
      GetDataContractResponse: PBJSGetDataContractResponse,
      GetDataContractsRequest: PBJSGetDataContractsRequest,
      GetDataContractsResponse: PBJSGetDataContractsResponse,
      GetDocumentsRequest: PBJSGetDocumentsRequest,
      GetDocumentsResponse: PBJSGetDocumentsResponse,
      GetIdentitiesByPublicKeyHashesResponse: PBJSGetIdentitiesByPublicKeyHashesResponse,
      GetIdentitiesByPublicKeyHashesRequest: PBJSGetIdentitiesByPublicKeyHashesRequest,
      GetIdentityByPublicKeyHashesResponse: PBJSGetIdentityByPublicKeyHashesResponse,
      GetIdentityByPublicKeyHashesRequest: PBJSGetIdentityByPublicKeyHashesRequest,
      WaitForStateTransitionResultRequest: PBJSWaitForStateTransitionResultRequest,
      WaitForStateTransitionResultResponse: PBJSWaitForStateTransitionResultResponse,
      GetConsensusParamsRequest: PBJSGetConsensusParamsRequest,
      GetConsensusParamsResponse: PBJSGetConsensusParamsResponse,
      GetDataContractHistoryRequest: PBJSGetDataContractHistoryRequest,
      GetDataContractHistoryResponse: PBJSGetDataContractHistoryResponse,
      GetProofsRequest: PBJSGetProofsRequest,
      GetProofsResponse: PBJSGetProofsResponse,
      GetEpochsInfoRequest: PBJSGetEpochsInfoRequest,
      GetEpochsInfoResponse: PBJSGetEpochsInfoResponse,
      GetVersionUpgradeVoteStatusRequest: PBJSGetVersionUpgradeVoteStatusRequest,
      GetVersionUpgradeVoteStatusResponse: PBJSGetVersionUpgradeVoteStatusResponse,
      GetVersionUpgradeStateRequest: PBJSGetVersionUpgradeStateRequest,
      GetVersionUpgradeStateResponse: PBJSGetVersionUpgradeStateResponse,
    },
  },
} = require('@dashevo/dapi-grpc');

const log = require('../../../log');

const createGrpcErrorFromDriveResponse = require('../createGrpcErrorFromDriveResponse');

const getIdentityHandlerFactory = require(
  './getIdentityHandlerFactory',
);
const getIdentitiesHandlerFactory = require(
  './getIdentitiesHandlerFactory',
);
const getIdentityBalanceHandlerFactory = require(
  './getIdentityBalanceHandlerFactory',
);
const getIdentityKeysHandlerFactory = require(
  './getIdentityKeysHandlerFactory',
);
const getIdentityBalanceAndRevisionHandlerFactory = require(
  './getIdentityBalanceAndRevisionHandlerFactory',
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
const getDataContractsHandlerFactory = require(
  './getDataContractsHandlerFactory',
);
const getDataContractHistoryHandlerFactory = require(
  './getDataContractHistoryHandlerFactory',
);
const getIdentityByPublicKeyHashesHandlerFactory = require(
  './getIdentityByPublicKeyHashesHandlerFactory',
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
const getProofsHandlerFactory = require(
  './getProofsHandlerFactory',
);
const getEpochsInfoHandlerFactory = require(
  './getEpochsInfoHandlerFactory',
);
const getVersionUpgradeVoteStatusHandlerFactory = require(
  './getVersionUpgradeVoteStatusHandlerFactory',
);
const getVersionUpgradeStateHandlerFactory = require(
  './getVersionUpgradeStateHandlerFactory',
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

  // getIdentities
  const getIdentitiesHandler = getIdentitiesHandlerFactory(
    driveClient,
  );

  const wrappedGetIdentities = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentitiesRequest,
      PBJSGetIdentitiesRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentitiesResponse,
    ),
    wrapInErrorHandler(getIdentitiesHandler),
  );

  // getIdentityBalance
  const getIdentityBalanceHandler = getIdentityBalanceHandlerFactory(
    driveClient,
  );

  const wrappedGetIdentityBalance = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityRequest,
      PBJSGetIdentityRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityBalanceResponse,
    ),
    wrapInErrorHandler(getIdentityBalanceHandler),
  );

  // getIdentityBalanceAndRevision
  const getIdentityBalanceAndRevisionHandler = getIdentityBalanceAndRevisionHandlerFactory(
    driveClient,
  );

  const wrappedGetIdentityBalanceAndRevision = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityRequest,
      PBJSGetIdentityRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityBalanceAndRevisionResponse,
    ),
    wrapInErrorHandler(getIdentityBalanceAndRevisionHandler),
  );

  // getIdentityKeys
  const getIdentityKeysHandler = getIdentityKeysHandlerFactory(
    driveClient,
  );

  const wrappedGetIdentityKeys = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityKeysRequest,
      PBJSGetIdentityKeysRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityKeysResponse,
    ),
    wrapInErrorHandler(getIdentityKeysHandler),
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

  // getDataContracts
  const getDataContractsHandler = getDataContractsHandlerFactory(
    driveClient,
  );

  const wrappedGetDataContracts = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetDataContractsRequest,
      PBJSGetDataContractsRequest,
    ),
    protobufToJsonFactory(
      PBJSGetDataContractsResponse,
    ),
    wrapInErrorHandler(getDataContractsHandler),
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

  // getIdentityByPublicKeyHashes
  const getIdentityByPublicKeyHashesHandler = getIdentityByPublicKeyHashesHandlerFactory(
    driveClient,
  );

  const wrappedGetIdentityByPublicKeyHashes = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityByPublicKeyHashesRequest,
      PBJSGetIdentityByPublicKeyHashesRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityByPublicKeyHashesResponse,
    ),
    wrapInErrorHandler(getIdentityByPublicKeyHashesHandler),
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

  // getProofs
  const getProofsHandler = getProofsHandlerFactory(
    driveClient,
  );

  const wrappedGetProofs = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetProofsRequest,
      PBJSGetProofsRequest,
    ),
    protobufToJsonFactory(
      PBJSGetProofsResponse,
    ),
    wrapInErrorHandler(getProofsHandler),
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

  // getVersionUpgradeVoteStatus
  const getVersionUpgradeVoteStatusHandler = getVersionUpgradeVoteStatusHandlerFactory(
    driveClient,
  );

  const wrappedGetVersionUpgradeVoteStatus = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetVersionUpgradeVoteStatusRequest,
      PBJSGetVersionUpgradeVoteStatusRequest,
    ),
    protobufToJsonFactory(
      PBJSGetVersionUpgradeVoteStatusResponse,
    ),
    wrapInErrorHandler(getVersionUpgradeVoteStatusHandler),
  );

  // getVersionUpgradeState
  const getVersionUpgradeStateHandler = getVersionUpgradeStateHandlerFactory(
    driveClient,
  );

  const wrappedGetVersionUpgradeState = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetVersionUpgradeStateRequest,
      PBJSGetVersionUpgradeStateRequest,
    ),
    protobufToJsonFactory(
      PBJSGetVersionUpgradeStateResponse,
    ),
    wrapInErrorHandler(getVersionUpgradeStateHandler),
  );

  return {
    broadcastStateTransition: wrappedBroadcastStateTransition,
    getIdentity: wrappedGetIdentity,
    getIdentities: wrappedGetIdentities,
    getIdentityBalance: wrappedGetIdentityBalance,
    getIdentityBalanceAndRevision: wrappedGetIdentityBalanceAndRevision,
    getIdentityKeys: wrappedGetIdentityKeys,
    getDocuments: wrappedGetDocuments,
    getDataContract: wrappedGetDataContract,
    getDataContracts: wrappedGetDataContracts,
    getDataContractHistory: wrappedGetDataContractHistory,
    getIdentityByPublicKeyHashes: wrappedGetIdentityByPublicKeyHashes,
    getIdentitiesByPublicKeyHashes: wrappedGetIdentitiesByPublicKeyHashes,
    waitForStateTransitionResult: wrappedWaitForStateTransitionResult,
    getConsensusParams: wrappedGetConsensusParams,
    getProofs: wrappedGetProofs,
    getEpochsInfo: wrappedGetEpochsInfo,
    getVersionUpgradeVoteStatus: wrappedGetVersionUpgradeVoteStatus,
    getVersionUpgradeState: wrappedGetVersionUpgradeState,
  };
}

module.exports = platformHandlersFactory;
