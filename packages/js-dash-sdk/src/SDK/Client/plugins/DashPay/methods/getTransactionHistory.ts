export async function getTransactionHistory(this: any){
  const account = await this.platform.client.getWalletAccount();
  const transactionHistory = account.getTransactionHistory();

  const { contacts } = this;

  contacts.forEach((contact)=>{
    const receivingAddresses = contact.getWatchedAddresses('receiving');
    const sendingAddresses = contact.getWatchedAddresses('sending');

    // TODO: Not for prod, only for draft purpose.
    transactionHistory.forEach((transactionItem)=>{
      transactionItem.from.forEach((fromItem)=>{
        if(receivingAddresses.includes(fromItem.address)){
          transactionItem.addressType = 'external'
          transactionItem.contact = contact;
        }
        if(sendingAddresses.includes(fromItem.address)){
          transactionItem.addressType = 'contact'
          transactionItem.contact = contact;
        }
      })
      transactionItem.to.forEach((toItem)=>{
        if(receivingAddresses.includes(toItem.address)){
          transactionItem.addressType = 'contact'
          transactionItem.contact = contact;
        }
        if(sendingAddresses.includes(toItem.address)){
          transactionItem.addressType = 'contact'
          transactionItem.contact = contact;
        }
      })
    })
  });

  return transactionHistory;
}
