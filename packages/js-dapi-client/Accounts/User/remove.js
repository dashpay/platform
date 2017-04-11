const has = require('../../util/has.js');
const {uuid}=require('khal');

exports.remove = function() {
    let self = this;
    return async function(query){
        return new Promise(function (resolve, reject) {
            let res = {};
            console.log(query);
            return resolve(res);
        });
    }
}