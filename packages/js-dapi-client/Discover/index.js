const Masternode = require('./Masternode/').Masternode;
exports.Discover = function(){
    let self = this;
    return {
        Masternode: Masternode.call(self),
        getInsightCandidate:require('./getInsightCandidate').getInsightCandidate.call(self),
        init:require('./init').init.call(self)
    };
};