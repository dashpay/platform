exports.Blockchain = function(){
    let self = this;
    return {
        restore:require('./restore').restore.call(self),
        _normalizeHeader:require('./util/_normalizeHeader'),
        init:require('./init').init.call(self),
        expectNextDifficulty:require('./expectNextDifficulty').expectNextDifficulty.call(self),
        addBlock:require('./addBlock').addBlock.call(self),
        getBlock:require('./getBlock').getBlock.call(self),
        getLastBlock:require('./getLastBlock').getLastBlock.call(self)
    };
};