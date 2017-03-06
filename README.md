[![Build Status](https://travis-ci.org/remram44/dhstore.svg?branch=master)](https://travis-ci.org/remram44/dhstore/builds)

[Generated documentation](https://remram44.github.io/adler32-rs/index.html)

What is this?
=============

This is very early work. I am trying to write a decentralized content management system (think Git, not Wordpress) in the Rust language.

Borrowing ideas from [IPFS](https://ipfs.io/) and [Camlistore](https://camlistore.org/), my objective is to write software that can store and index objects, offering replication, sharing and search over multiple nodes.

Use cases:
* Store your backups in an efficient (de-duplicated) manner, and replicate them to different machines
* Index and tag your photos, allowing for efficient search by tag/date/location
* Ingest your Tweets and social media posts for posterity and easy search
* Wiki-like application with rich-text notes linking forward and back to images, files, emails, ...
* Persistent decentralized archive (archive.org mirror?)
* Backend for all kinds of applications (calendar, website, microblogging)
