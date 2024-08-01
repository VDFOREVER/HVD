<h1 align="center" style="font-weight: 900; font-size: 30px;" >About</h1>
Telegram bot for tracking new posts by tags with rule34 gelbooru, kemono, with the ability to filter

<h1 align="center" style="font-weight: 900; font-size: 30px;" >Settings</h1>

In the .env file, specify the telegram bot token in TELOXIDE_TOKEN and in the ADMIN item, specify the id of your telegram account. An account with admin rights can add and delete bot users

example:
```
TELOXIDE_TOKEN=fdgdfgdfghdjhjkrhtjeo
ADMIN=0000000
```
<h1 align="center" style="font-weight: 900; font-size: 30px;" >Guide</h1>

After launching the bot you need to configure it, first you need to add yourself or someone else as a user to use the bot

Admin Command:

```
/adduser <id>
/rmuser <id>
```

After adding a user, he/she has access to commands

User Command:

```
/help
```

Add/remove tag to search for new posts
```
/addtag <service> <tag>
/rmtag <service> <tag>
```
Add/Remove tag with which posts will not be sent to you
```
/addantitag <service> <tag>
/rmantitag <service> <tag>
```
list of tags you added
```
/taglist
```

**Please note the way to add tags for kemono is different from others**
```
/addtag kemono https://kemono.su/api/v1/<service>/user/<id>
/rmtag kemono https://kemono.su/api/v1/<service>/user/<id>
```
example:
```
https://kemono.su/fanbox/user/000000
fanbox - service
000000 - id

/addtag kemono https://kemono.su/api/v1/fanbox/user/000000
```