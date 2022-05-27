const DashPlatformProtocol = require('./DashPlatformProtocol');

const Identity = require('./identity/Identity');
const IdentityPublicKey = require('./identity/IdentityPublicKey');
const Identifier = require('./identifier/Identifier');

const DataContractFactory = require('./dataContract/DataContractFactory');

const InvalidDataContractVersionError = require('./errors/consensus/basic/dataContract/InvalidDataContractVersionError');
const IncompatibleDataContractSchemaError = require('./errors/consensus/basic/dataContract/IncompatibleDataContractSchemaError');
const IdentityNotFoundError = require('./errors/consensus/signature/IdentityNotFoundError');

DashPlatformProtocol.DataContractFactory = DataContractFactory;

DashPlatformProtocol.Identity = Identity;
DashPlatformProtocol.IdentityPublicKey = IdentityPublicKey;
DashPlatformProtocol.Identifier = Identifier;

DashPlatformProtocol.Errors = {
  IncompatibleDataContractSchemaError,
  IdentityNotFoundError,
  InvalidDataContractVersionError,
};

module.exports = DashPlatformProtocol;
