exports.getLastDifficulty = function() {
    let self = this;
    return async function(){
        return new Promise(function (resolve, reject) {
            return self.Explorer.API.getStatus().then(function (_status) {
                return resolve(_status.info.difficulty);
            });
        });
    }
}