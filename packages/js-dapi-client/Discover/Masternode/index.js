const Masternode = function(){
    let self = this;
    return {
        fetcher:require('./fetcher').fetcher.call(self),
        validate:require('./validate').validate.call(self),
    }
};

exports.Masternode = Masternode;