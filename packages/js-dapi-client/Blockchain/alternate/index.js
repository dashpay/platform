exports.Blockchain = function() {
    let self = this;
    return {
        init: require('./init').init
    };
};