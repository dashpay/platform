const {clone} = require('khal');
const Buffer = require('../util/Buffer');
const DashUtil = require('dash-util')
const Bitcore = require('bitcore-lib-dash');

exports.addBlock = function() {
    let self = this;
    return async function(blocks){
        return new Promise(async function (resolve, reject) {
            let listOfHeader = [];
            if(!Array.isArray(blocks)){
                listOfHeader.push(blocks);
            }else{
                listOfHeader=blocks;
            }
            listOfHeader = listOfHeader.map(function(_bh){
                return self.Blockchain._normalizeHeader(_bh)
            });
            self.Blockchain.chain.addHeaders(listOfHeader,function(err){
                if(err) console.error(err);
                return resolve(true);
            });
            // return resolve(true);
        });
    }
}