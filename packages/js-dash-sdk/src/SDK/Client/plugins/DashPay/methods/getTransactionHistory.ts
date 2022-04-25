export async function getTransactionHistory(this: any){
  const account = await this.platform.client.getWalletAccount();
  const transactionHistory = account.getTransactionHistory();

  const { contacts } = this;

  contacts.forEach((contact)=>{
    const receivingAddresses = contact.getWatchedAddresses('receiving');
    const sendingAddresses = contact.getWatchedAddresses('sending');

    transactionHistory.forEach((transactionItem)=>{
      transactionItem.from.forEach((fromItem)=>{
        if(receivingAddresses.includes(fromItem.address)){
          fromItem.addressType = 'external'
          fromItem.contact = contact;
        }
        if(sendingAddresses.includes(fromItem.address)){
          fromItem.addressType = 'contact'
          fromItem.contact = contact;
        }
      })
      transactionItem.to.forEach((toItem)=>{
        if(receivingAddresses.includes(toItem.address)){
          toItem.addressType = 'contact'
          toItem.contact = contact;
        }
        if(sendingAddresses.includes(toItem.address)){
          toItem.addressType = 'contact'
          toItem.contact = contact;
        }
      })
    })
  });

  return transactionHistory;
}
