exports.getLastDifficulty = function() {
    return new Promise(function(resolve, reject) {
        return SDK.Explorer.API.getStatus()
            .then(function(_status) {
                resolve(_status.info.difficulty);
            })
            .catch(err => reject(err))
    });
}
