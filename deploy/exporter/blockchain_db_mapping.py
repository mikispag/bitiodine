from sqlalchemy import Column, Integer, BigInteger, String, ForeignKey
from sqlalchemy.orm import relationship, backref
from sqlalchemy.ext.declarative import declarative_base
Base = declarative_base()


class Block(Base):
    __tablename__ = 'blocks'

    block_id = Column(BigInteger, primary_key=True)
    block_hash = Column(String)
    time = Column(BigInteger)

    def __repr__ (self):
        return "block_id: "+str(self.block_id)+" block_hash: "+self.block_hash

class Tx(Base):
    __tablename__ = 'tx'

    tx_id = Column(BigInteger, primary_key=True)
    tx_hash = Column(String)
    block_id = Column(BigInteger,ForeignKey('blocks.block_id'))
    
    block = relationship("Block", backref=backref('txs', order_by=tx_id))

    def __repr__ (self):
        return "tx_id: "+str(self.tx_id)+" tx_hash: "+self.tx_hash+" block_id: "+str(self.block_id)

    
class TxIn(Base):
    __tablename__ = 'txin'
    
    txin_id = Column(BigInteger,primary_key=True)
    txout_id = Column(BigInteger, ForeignKey('txout.txout_id'))
    tx_id = Column(BigInteger, ForeignKey('tx.tx_id'))
    txin_pos = Column(Integer)

    tx = relationship("Tx", backref=backref('txIns', order_by=txin_pos))
    txout = relationship("TxOut")

    def __repr__ (self):
       return "txin_id: "+str(self.txin_id)+" txout_id: "+str(self.txout_id)
    

class TxOut(Base):
    __tablename__ = 'txout'

    txout_id = Column(BigInteger,primary_key=True)
    address = Column(String)
    txout_value = Column(BigInteger)
    tx_id = Column(BigInteger, ForeignKey('tx.tx_id'))
    txout_pos = Column(Integer)

    tx = relationship("Tx", backref=backref('txOuts', order_by=txout_pos))

    def __repr__ (self):
        return "txout_id: "+str(self.txout_id)+" address: "+self.address+" value: "+str(self.txout_value)


    
    

