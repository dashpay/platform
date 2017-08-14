exports.restore = function() {
    let self = this;
    return async function(query, update) {
        return new Promise(function(resolve, reject) {
            return resolve(true);
        });
    }
}