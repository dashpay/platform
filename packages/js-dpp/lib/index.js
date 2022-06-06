const DashPlatformProtocol = require('./DashPlatformProtocol');

const Identity = require('./identity/Identity');
const IdentityPublicKey = require('./identity/IdentityPublicKey');
const Identifier = require('./identifier/Identifier');

const DataContractFactory = require('./dataContract/DataContractFactory');

const InvalidDataContractVersionError = require('./errors/consensus/basic/dataContract/InvalidDataContractVersionError');
const IncompatibleDataContractSchemaError = require('./errors/consensus/basic/dataContract/IncompatibleDataContractSchemaError');
const InvalidInstantAssetLockProofSignatureError = require('./errors/consensus/basic/identity/InvalidInstantAssetLockProofSignatureError');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('./errors/consensus/basic/identity/IdentityAssetLockTransactionOutPointAlreadyExistsError');
const InvalidDocumentTypeError = require('./errors/consensus/basic/document/InvalidDocumentTypeError');
const IdentityNotFoundError = require('./errors/consensus/signature/IdentityNotFoundError');
const BalanceIsNotEnoughError = require('./errors/consensus/fee/BalanceIsNotEnoughError');

const protocolVersion = require('./version/protocolVersion');

DashPlatformProtocol.DataContractFactory = DataContractFactory;

DashPlatformProtocol.Identity = Identity;
DashPlatformProtocol.IdentityPublicKey = IdentityPublicKey;
DashPlatformProtocol.Identifier = Identifier;

DashPlatformProtocol.version = protocolVersion.latestVersion;

DashPlatformProtocol.Errors = {
  BalanceIsNotEnoughError,
  IncompatibleDataContractSchemaError,
  IdentityNotFoundError,
  InvalidDataContractVersionError,
  InvalidDocumentTypeError,
  InvalidInstantAssetLockProofSignatureError,
  IdentityAssetLockTransactionOutPointAlreadyExistsError,
};

module.exports = DashPlatformProtocol;
