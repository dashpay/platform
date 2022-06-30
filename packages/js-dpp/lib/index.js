const DashPlatformProtocol = require('./DashPlatformProtocol');

const Identity = require('./identity/Identity');
const IdentityPublicKey = require('./identity/IdentityPublicKey');
const Identifier = require('./identifier/Identifier');

const DataContractFactory = require('./dataContract/DataContractFactory');

const consensusErrorCodes = require('./errors/consensus/codes');

const protocolVersion = require('./version/protocolVersion');

DashPlatformProtocol.DataContractFactory = DataContractFactory;

DashPlatformProtocol.Identity = Identity;
DashPlatformProtocol.IdentityPublicKey = IdentityPublicKey;
DashPlatformProtocol.Identifier = Identifier;

DashPlatformProtocol.version = protocolVersion.latestVersion;

DashPlatformProtocol.ConsensusErrors = Object.values(consensusErrorCodes)
  .reduce((obj, ConsensusErrorClass) => {
    // eslint-disable-next-line no-param-reassign
    obj[ConsensusErrorClass.name] = ConsensusErrorClass;

    return obj;
  }, {});

module.exports = DashPlatformProtocol;
