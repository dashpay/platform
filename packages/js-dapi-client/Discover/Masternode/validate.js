const has = require('../../util/has.js');
const requesterJSON = require('../../util/requesterJSON.js');
const { uuid } = require('khal');

isPingable = function(el) {
    return new Promise(function(resolve, reject) {
        let uri = el.fullBase + el.insightPath + '/status';
        requesterJSON.get(uri)
            .then(function(resp) {
                if ((resp && resp.hasOwnProperty('info'))) {
                    resolve(el);
                } else {
                    //pvr: some error handling
                }
            })
            .catch(function(err) {
                console.log(err);
            });
    })
};
exports.validate = function(_unvalidList) {

    return new Promise(function(resolve, reject) {
        Promise.all(_unvalidList.map(ul => isPingable(ul)))
            .then(validList => {
                resolve(validList)
            })
    });
}