const has = require('../../util/has.js');
const requesterJSON = require('../../util/requesterJSON.js');
const {uuid}=require('khal');


const isPingable = function(el){
    return new Promise(function (resolve, reject) {
        let uri = el.fullBase+el.insightPath+'/status';
        requesterJSON
            .get(uri)
            .then(function (resp) {
                if ((resp && resp.hasOwnProperty('info'))) {
                    return resolve(true);
                }else{
                    return resolve(false);
                }
            })
            .catch(function (err) {
                return resolve(false);
            });
    })
};
exports.validate = function() {
    let self = this;
    return async function (_unvalidList) {
        return new Promise(async function (resolve, reject) {
            let validList = [];
            for(let i =0; i<_unvalidList.length; i++){
                if(await isPingable(_unvalidList[i])){
                    validList.push(_unvalidList[i]);
                }else{
                    if(self._config.verbose) console.log('Not valid found', _unvalidList[i]);
                }
            }
            return resolve(validList);
        });
    }
}