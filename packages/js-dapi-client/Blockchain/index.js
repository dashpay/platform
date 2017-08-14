exports.Blockchain = function() {
    let self = this;
    return {
        restore: require('./restore').restore,
        _normalizeHeader: require('./util/_normalizeHeader'),
        init: require('./init').init,
        expectNextDifficulty: require('./expectNextDifficulty').expectNextDifficulty,
        addBlock: require('./addBlock').addBlock,
        getBlock: require('./getBlock').getBlock,
        getLastBlock: require('./getLastBlock').getLastBlock
    };
};