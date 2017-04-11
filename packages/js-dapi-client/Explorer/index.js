exports.Explorer = function(){
    let self = this;
    return {
        API:{
            getStatus:require('./insightAPI/getStatus').getStatus.call(self),
            getBlock:require('./insightAPI/getBlock').getBlock.call(self),
            getLastBlockHeight:require('./insightAPI/getLastBlockHeight').getLastBlockHeight.call(self),
            getLastDifficulty:require('./insightAPI/getLastDifficulty').getLastDifficulty.call(self),
            getHashFromHeight:require('./insightAPI/getHashFromHeight').getHashFromHeight.call(self),
            getHeightFromHash:require('./insightAPI/getHeightFromHash').getHeightFromHash.call(self),
        }
    };
};