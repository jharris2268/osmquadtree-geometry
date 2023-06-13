use postgres::{Client,NoTls};
use std::io::{Error,ErrorKind,Result,Write};
use std::cell::RefCell;

pub struct Connection {
    client: RefCell<Client>,
}

fn wrap_postgres_error<T>(x: core::result::Result<T,postgres::Error>) -> Result<T> {
    match x {
        Ok(t) => Ok(t),
        Err(e) => Err(Error::new(ErrorKind::Other, format!("{:?}",e)))
    }
}

impl Connection {
    pub fn connect(connstr: &str) -> Result<Connection> {
        
        Ok(Connection{client: RefCell::new(wrap_postgres_error(Client::connect(connstr, NoTls))?)})
    }
    
    pub fn execute(&self, sql: &str) -> Result<()> {
        let mut client = self.client.borrow_mut();
        wrap_postgres_error(client.execute(sql, &[]))?;
        Ok(())
    }
    
    pub fn copy(&self, cmd: &str, data: &[&[u8]]) -> Result<()> {
        let mut client = self.client.borrow_mut();
        let mut writer = wrap_postgres_error(client.copy_in(cmd))?;
        
        for d in data {
            writer.write_all(d)?;
        }
        wrap_postgres_error(writer.finish())?;
        Ok(())
            
    }
}
