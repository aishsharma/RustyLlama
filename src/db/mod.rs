use rusqlite::{Connection, Result};
use rusqlite_migration::{Migrations, M};
use std::path::Path;
use crate::models::{Conversation, Message};
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        
        let migrations = Migrations::new(vec![
            M::up("CREATE TABLE conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                model TEXT NOT NULL,
                system_prompt TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"),
            M::up("CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                token_count INTEGER,
                FOREIGN KEY (conversation_id) REFERENCES conversations (id)
            );"),
        ]);
        
        migrations.to_latest(&mut conn)?;
        
        Ok(Self { conn })
    }
    
    pub fn save_conversation(&self, conv: &Conversation) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO conversations (id, title, model, system_prompt, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (&conv.id.to_string(), &conv.title, &conv.model, &conv.system_prompt, &conv.created_at.to_rfc3339(), &conv.updated_at.to_rfc3339()),
        )?;
        
        // Save messages
        for msg in &conv.messages {
            self.conn.execute(
                "INSERT OR REPLACE INTO messages (id, conversation_id, role, content, timestamp, token_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (&msg.id.to_string(), &conv.id.to_string(), &msg.role, &msg.content, &msg.timestamp.to_rfc3339(), &msg.token_count),
            )?;
        }
        
        Ok(())
    }
    
    pub fn load_conversations(&self) -> Result<Vec<Conversation>> {
        let mut stmt = self.conn.prepare("SELECT id, title, model, system_prompt, created_at, updated_at FROM conversations ORDER BY updated_at DESC")?;
        let conv_iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let model: String = row.get(2)?;
            let system_prompt: String = row.get(3)?;
            let created_at: String = row.get(4)?;
            let updated_at: String = row.get(5)?;
            
            Ok((Uuid::parse_str(&id).unwrap(), title, model, system_prompt, DateTime::parse_from_rfc3339(&created_at).unwrap().with_timezone(&Utc), DateTime::parse_from_rfc3339(&updated_at).unwrap().with_timezone(&Utc)))
        })?;
        
        let mut conversations = vec![];
        for conv_result in conv_iter {
            let (id, title, model, system_prompt, created_at, updated_at) = conv_result?;
            
            // Load messages
            let mut msg_stmt = self.conn.prepare("SELECT id, role, content, timestamp, token_count FROM messages WHERE conversation_id = ? ORDER BY timestamp")?;
            let msg_iter = msg_stmt.query_map([&id.to_string()], |row| {
                let msg_id: String = row.get(0)?;
                let role: String = row.get(1)?;
                let content: String = row.get(2)?;
                let timestamp: String = row.get(3)?;
                let token_count: Option<usize> = row.get(4)?;
                
                Ok(Message {
                    id: Uuid::parse_str(&msg_id).unwrap(),
                    role,
                    content,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp).unwrap().with_timezone(&Utc),
                    token_count,
                })
            })?;
            
            let messages: Vec<Message> = msg_iter.collect::<Result<_>>()?;
            
            conversations.push(Conversation {
                id,
                title,
                model,
                system_prompt,
                messages,
                created_at,
                updated_at,
            });
        }
        
        Ok(conversations)
    }
    
    pub fn delete_conversation(&self, id: &Uuid) -> Result<()> {
        self.conn.execute("DELETE FROM messages WHERE conversation_id = ?", [&id.to_string()])?;
        self.conn.execute("DELETE FROM conversations WHERE id = ?", [&id.to_string()])?;
        Ok(())
    }
}