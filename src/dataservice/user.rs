
use rustorm::DbError;
use crate::db;
use crate::model::Ruser;
use crate::model::{for_write, for_read};
use crate::util::{random_string, sha3_256_encode};
use redis::Commands;
use chrono::{DateTime, Utc};

use log::info;


pub fn set_session(account: &str, ttl: usize) -> Result<String, String> {
    let redis = db::get_redis();
    let cookie = sha3_256_encode(&random_string(8));
    let _: () = redis.hset(&cookie, "login_time", Utc::now().timestamp()).unwrap();
    let _: () = redis.hset(&cookie, "account", account).unwrap();
    let _: () = redis.expire(&cookie, ttl).unwrap();

    Ok(cookie)
}


// this struct is defined for request params
pub struct UserSignUp {
    pub account: String,
    pub password: String,
    pub nickname: String,
}

impl UserSignUp {
    pub fn sign_up_with_email (&self) -> Result<Ruser, String>{
        let em = db::get_db();
        let salt = random_string(6);

        let new_user = for_write::Ruser {
            account: self.account.to_owned(),
            password: sha3_256_encode(&format!("{}{}", self.password, salt)),
            salt: salt,
            nickname: self.nickname.to_owned(),
            github: None,
        };

        let rest_clause = format!("WHERE account='{}'", new_user.account); 
        // check if the same name account exists already 
        match db_select!(em, "", "", &rest_clause, Ruser).first() {
            Some(_) => {
                // exist already, return Error
                Err(format!("user {} exists.", new_user.account))
            },
            None => {
                // it's a new user, insert it
                match db_insert!(em, &new_user, Ruser) {
                    Some(user) => {
                        // generate a corresponding section to this user as his blog section
                        let section = for_write::Section {
                            title: user.nickname.to_owned(),
                            description: format!("{}'s blog", user.nickname),
                            stype: 1,
                            suser: Some(user.id.to_owned()),
                        };
                        section.insert();

                        // set user cookies to redis to keep login session
                        set_session(&user.account, 24*3600).unwrap();

                        Ok(user.to_owned())
                    },
                    None => {
                        unreachable!();
                    }
                }
            }
        }
    }
}



impl for_write::UserEdit {
    
    pub fn edit(&self, cookie: &str) -> Result<Ruser, String> {
        let em = db::get_db();
        let redis = db::get_redis();
        let account: String = redis.hget(cookie, "account").unwrap();

        // update new info by account
        match db_update!(em, self, &format!("WHERE account={}", account), Ruser) {
            Some(user) => {
                Ok(user.to_owned())
            },
            None => {
                Err("User doesn't exist.".into())
            }
        }
    }

}


pub struct UserLogin {
    account: String,
    password: String,
    remember: bool,
}

impl UserLogin {

    pub fn verify_login(&self, max_age: &Option<usize>) -> Result<String, String> {
        let em = db::get_db();

        let rest_clause = format!("WHERE status=0 and account='{}'", self.account); 
        // check if the same name account exists already 
        match db_select!(em, "", "", &rest_clause, Ruser).first() {
            Some(user) => {
                // check calulation equality
                if user.password == sha3_256_encode(&format!("{}{}", self.password, user.salt)) {
                    let ttl = match *max_age {
                        Some(t) => t * 3600,
                        None => 24 * 60 * 60,
                    };
                    
                    // store session
                    set_session(&self.account, ttl)

                } else {
                    Err("Wrong account or password.".into())
                }

            },
            None => {
                Err("User doesn't exist.".into())
            }
        }

    }

}

pub struct UserChangePassword {
    pub old_password: String,
    pub new_password: String,
}

impl UserChangePassword {


}



impl Ruser {

    pub fn sign_out(cookies: &str) -> Result<(), String> {
        let redis = db::get_redis();
        let _: () = redis.del(cookies).unwrap();

        Ok(())
    }

}




pub fn test () {
    let em = db::get_db();
    let sql = "SELECT * FROM ruser LIMIT 10";
    let users: Result<Vec<Ruser>, DbError> = em.execute_sql_with_return(sql, &[]);
    let users = users.unwrap();
    assert_eq!(users.len(), 1);
    for user in users {
        info!("user: {:?}", user);
    }
}


