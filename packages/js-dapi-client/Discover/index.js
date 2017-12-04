const Masternode = require('./Masternode/').Masternode;
exports.Discover = function() {
    let self = this;
    return {
        _state: "waiting",
        Masternode: Masternode(),
        getConnectorCandidateURI: require('./getConnectorCandidateURI').getConnectorCandidateURI,
        getSocketCandidate: require('./getSocketCandidate').getSocketCandidate,
    };
};