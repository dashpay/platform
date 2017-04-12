exports.Blockchain = function(){
    let self = this;
    return {
        restore:require('./restore').restore.call(self),
        init:require('./init').init.call(self),
        expectNextDifficulty:require('./expectNextDifficulty').expectNextDifficulty.call(self)
};
};