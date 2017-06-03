const Masternode = require('./Masternode/').Masternode;
exports.Discover = function() {
    let self = this;
    return {
        _state: "waiting",
        Masternode: Masternode(),
        getInsightCandidate: require('./getInsightCandidate').getInsightCandidate,
        getInsightCandidateURI: require('./getInsightCandidateURI').getInsightCandidateURI,
        getSocketCandidate: require('./getSocketCandidate').getSocketCandidate,
        init: require('./init').init
    };
};