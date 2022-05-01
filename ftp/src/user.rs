use std::collections::HashMap;

pub type Username = String;
pub type Password = String;

pub struct User {
    pub password: Password,
    pub dir: String,
}

pub type UserStore = HashMap<Username, User>;
