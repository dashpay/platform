const Masternode = function(){
    let self = this;
    return {
        fetcher:require('./fetcher').fetcher,
        validate:require('./validate').validate,
    }
};

exports.Masternode = Masternode;