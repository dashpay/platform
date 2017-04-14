//Choose a random insight uri
const {math} = require('khal');

exports.getInsightCandidate = function() {
    let self = this;
    return async function(){
        let args = arguments;
        return new Promise(async function (resolve, reject) {
            if(self.Discover._state!=="ready"){
                await self.Discover.init();
                return resolve(self.Discover.getInsightCandidate.apply(null, args));
            }
                // throw new Error('Discover need to be init first');

            let validMNList = self.Discover.Masternode.validMNList;
            if(validMNList && validMNList.length>0){
                //Select randomnly one of them
                let selectedMNIdx = math.randomBetweenMinAndMax(0, validMNList.length-1);
                let el = validMNList[selectedMNIdx];
                return resolve({URI:el.fullBase+el.insightPath,idx:selectedMNIdx});
            }else{
                throw new Error('No MN found :( Sadness & emptyness');
                return resolve(false);
            }

        });
    }
}