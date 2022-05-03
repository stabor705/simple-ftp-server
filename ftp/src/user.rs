pub type Username = String;
pub type Password = String;

#[derive(Clone)]
pub struct User {
    pub username: String,
    pub data: UserData,
}

#[derive(Clone)]
pub struct UserData {
    pub password: Password,
    pub dir: String,
}
