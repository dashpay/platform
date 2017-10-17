//(semi) common code present in dapi + sdk (to be extracted to single lib)

const _ = require('lodash');
const quorumSize = 1;

module.exports = {
    getQuorum: function(list, blockHash, txid) {
        return GetQuorumForUser(list, blockHash, txid);
    },

    //tmp until rpc
    resolveVinFromIp: function(mnList, ip) {
        return _.find(mnList, mn => mn.ip = ip).vin;
    }
}

//Not conforming to DIP-022 yet, possible update after dip is complete

//1: to discuss - weakness in using blockhash as leading zeros will penalise/reward mns with vin collateral closer to extremes in the search space
//mitigate by stripping leading zeros and pad with equal amount from end of the hash?

//2: to discuss - weakness in same mn's in same quorums while for the same mnLists

//As per (1) remove zeros and pad with end to get a truely random value within the 256-bit search space
//We can also hash the blockhash for same effect with slightly more - albeit negligible - overhead for clients 
var GetTruelyRandomBlockHash = function(blockHash) {
    let leadingZeros = _.takeWhile(blockHash.split(""), e => e == '0').length;
    return blockHash.substring(blockHash.length - leadingZeros, blockHash.length) + blockHash.substring(leadingZeros, blockHash.length);
}

//XOR 2 64Byte hex strings
GetBitwiseXOR = function(hex1, hex2) {
    let hex1Arr = hex1.split("");
    let hex2Arr = hex2.split("");

    let result = "";

    for (let i = 0; i < 64; i++) {
        result += (parseInt(hex1Arr[i], 16) ^ parseInt(hex2Arr[i], 16)).toString(16);
    }

    return result;
}

var GetQuorumForUser = function(mnList, blockHash, txid) {

    //XOR blockhash and user regtx to get random position on search space influenced by lastblockhash and user
    let refHash = GetBitwiseXOR(GetTruelyRandomBlockHash(blockHash), txid);

    //sort mn's with vin closest to refHash listed first
    let sortedMnList = mnList.sort((mn1, mn2) => mn1.vin < mn2.vin);

    return sortedMnList.slice(0, quorumSize)
}

