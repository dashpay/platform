exports.expectNextDifficulty = function() {
    let self = this;
    return async function(){
        return new Promise(async function (resolve, reject) {
            return resolve(0);
        });
    }
}