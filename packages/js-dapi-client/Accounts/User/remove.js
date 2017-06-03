const has = require('../../util/has.js');
const { uuid } = require('khal');

exports.remove = function(query) {

    return new Promise(function(resolve, reject) {
        let res = {};
        console.log(query);
        resolve(true);
    });
}
