use std::io::Error;

use worker::console_log;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Serialize;


#[derive(Serialize, Debug)]
pub struct Article {
    pub title: String,
    pub link: String,
    pub time: String,
    pub author: String,
    pub sourcelink: String,
    pub sourcename: String,
    pub image_link: String
}

pub async fn scrape_website(url : &str) -> String {
   let client = Client::new();
   let mut res = client.get(url).send().await.unwrap();
   let body = res.text().await.unwrap();
   let document = Html::parse_document(&body);
   //Select the elements of type article
    let selector = Selector::parse("article").unwrap();
    let articles = document.select(&selector);
    for article in articles {
        console_log!("{:?}", article);
    }
    url.to_string()
}


pub async fn google_news_scraper() -> Result<Vec<Article>, Error> {
    let client = Client::new();
    let mut res = client.get("https://news.google.com/topics/CAAqJggKIiBDQkFTRWdvSUwyMHZNRFZxYUdjU0FtVnVHZ0pWVXlnQVAB").send().await.unwrap();
    let body = res.text().await.unwrap();
    let document = Html::parse_document(&body);
    let selector = Selector::parse("article").unwrap();
    let articles = document.select(&selector);
    let mut article_list = vec![];
    for article in articles {
        let title_selector = Selector::parse("a[data-n-tid='29']").unwrap();
        let title = match article.select(&title_selector).next() {
            Some(title) => title.text().collect::<Vec<_>>().join(" "),
            None => "".to_string()
        };
        let link = match article.select(&title_selector).next() {
            Some(link) => link.value().attr("href").unwrap().to_string(),
            None => "".to_string()
        };
        let time_selector = Selector::parse("time").unwrap();
        let time = match article.select(&time_selector).next() {
            Some(time) => time.value().attr("datetime").unwrap().to_string(),
            None => "".to_string()
        };
        //Author selector has class name "PJK1m"
        let author_selector = Selector::parse("span.PJK1m").unwrap();
        let mut author = match article.select(&author_selector).next() {
            Some(author) => author.text().collect::<Vec<_>>().join(" "),
            None => "".to_string()
        };
        if author.is_empty() {
            author = "Syndicated Source".to_string();
        }
        //Source Image selector has class name "qEdqNd"
        let sourcelink_selector = Selector::parse("img.qEdqNd").unwrap();
        let sourcelink = match article.select(&sourcelink_selector).next() {
            Some(sourcelink) => sourcelink.value().attr("src").unwrap().to_string(),
            None => "".to_string()
        };
        //Source name has div[data-n-tid='9']
        let source_name_selector = Selector::parse("div[data-n-tid='9']").unwrap();
        let sourcename = match article.select(&source_name_selector).next() {
            Some(sourcename) => sourcename.text().collect::<Vec<_>>().join(" "),
            None => "".to_string()
        };
        //If the article has figure tag, it has an image inside it with img tag with class Quavad
        let image_selector = Selector::parse("figure img.Quavad").unwrap();
        let image_link = match article.select(&image_selector).next() {
            Some(image_link) => image_link.value().attr("src").unwrap().to_string(),
            None => "".to_string()
        };
        let article = Article {
            title,
            link,
            time,
            author,
            sourcelink,
            sourcename,
            image_link
        };
        //If any of the fields are empty, we don't want to include the article
        if article.title.is_empty() || article.link.is_empty() || article.time.is_empty() || article.author.is_empty() || article.sourcelink.is_empty() || article.sourcename.is_empty() {
            continue;
        } else {
            article_list.push(article);
        }
    }
    //Get the first 10 if there are more than 10 articles
    if article_list.len() > 10 {
        article_list.truncate(10);
    }
    match article_list.len() {
        0 => Err(Error::new(std::io::ErrorKind::Other, "No articles found")),
        _ => Ok(article_list)
    }
}