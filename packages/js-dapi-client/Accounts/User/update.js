const has = require('../../util/has.js');
const {uuid}=require('khal');

exports.update = function() {
    let self = this;
    return async function(query, update){
        return new Promise(function (resolve, reject) {
            return resolve(true);
        });
    }
}