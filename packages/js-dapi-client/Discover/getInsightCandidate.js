//Choose a random insight uri
const {math} = require('khal');

exports.getInsightCandidate = function() {
    let self = this;
    return async function(query, update){
        return new Promise(async function (resolve, reject) {
            let validMNList = self.Discover.Masternode.validMNList;
            if(validMNList.length>0){
                //Select randomnly one of them
                let selectedMNIdx = math.randomBetweenMinAndMax(0, validMNList.length-1);
                return resolve({URI:validMNList[selectedMNIdx],idx:selectedMNIdx});
            }else{
                throw new Error('No MN found :( Sadness & emptyness');
                return resolve(false);
            }
        });
    }
}