use chrono::{Local, Utc};
use fake::uuid::{UUIDv4};
use fake::{Dummy, Fake, Faker};
use rand::rngs::StdRng;
use rand::SeedableRng;
use uuid::Uuid;
use fake::faker::name::en::*;
// use fake::locales::{EN, ZH_CN};
use fake::locales::{EN, FR_FR, ZH_CN};
// use fake::Fake;
use fake::faker::chrono::raw::*;
// use chrono::DateTime;
// use chrono::Local;
// use fake::locales::*;
// use fake::locales::ZH_CN;
// use fake::faker::name::zh_cn::Name;
mod domain;
// mod infrastructure;
// mod presentation;
// mod services;

#[derive(Debug, Dummy)]
pub struct Foo {
    #[dummy(faker = "UUIDv4")]
    order_id: Uuid,
    #[dummy(faker = "Name()")]
    customer: String,
    // #[dummy(faker = "fake::faker::lorem::en::Words(3..5)")]
    product: String,
    paid: bool,
}

fn main() {
    // type derived Dummy
    let f: Foo = Faker.fake();
    println!("{:?}", f);

    let f2 = Foo {
        order_id: Uuid::new_v4(),
        customer: Name(ZH_CN).fake(),
        product: Word().fake(),
        paid: Faker.fake::<bool>(),
    };
    println!("{:?}", f2);

    println!("{:?}", chrono::offset::Utc::now());
    println!("{:?}", chrono::offset::Local::now());

    let create_date: chrono::DateTime<Utc> = DateTimeBefore(EN, Utc::now()).fake();
    print!("Date: {:?}", create_date);


    // using `Faker` to generate default fake value of given type
    let tuple = Faker.fake::<(u8, u32, f32)>();
    println!("tuple {:?}", tuple);
    println!("String {:?}", Faker.fake::<String>());

    // types U can used to generate fake value T, if `T: Dummy<U>`
    println!("String {:?}", (8..20).fake::<String>());
    println!("u32 {:?}", (8..20).fake::<u32>());

    // using `faker` module with locales
    use fake::faker::name::raw::*;
    use fake::locales::*;

    let id: Uuid = UUIDv4.fake();
    println!("uuid {:?}", id);

    let name: String = Name(EN).fake();
    println!("name {:?}", name);

    let name: String = Name(ZH_CN).fake();
    println!("name {:?}", name);

    // using convenient function without providing locale
    use fake::faker::lorem::en::*;
    let words: Vec<String> = Words(3..5).fake();
    println!("words {:?}", words);

    // using macro to generate nested collection
    let name_vec = fake::vec![String as Name(EN); 4, 3..5, 2];
    println!("random nested vec {:?}", name_vec);

    // fixed seed rng
    let seed = [
        1, 0, 0, 0, 23, 0, 0, 0, 200, 1, 0, 0, 210, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    let ref mut r = StdRng::from_seed(seed);
    for _ in 0..5 {
        let v: usize = Faker.fake_with_rng(r);
        println!("value from fixed seed {}", v);
    }

    let val: String = Paragraph(3..5).fake();
    println!("{:?}", val);
}