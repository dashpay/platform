exports.Explorer = function(){
    let self = this;
    return {
        API:{
            getStatus:require('./insightAPI/getStatus').getStatus.call(self),
            getLastBlockHeight:require('./insightAPI/getLastBlockHeight').getLastBlockHeight.call(self),
            getLastDifficulty:require('./insightAPI/getLastDifficulty').getLastDifficulty.call(self),
        }
    };
};