use std::{
    io::Read,
    net::TcpStream,
    sync::mpsc,
    thread::{self, JoinHandle},
};

use crate::{
    error::CustomError,
    message::{Message, MessageHeader},
    messages::{
        block::Block,
        get_data::GetData,
        headers::Headers,
        inv::{Inventory, InventoryType},
        ping_pong::{Ping, Pong},
    },
    peer::{request_headers, NodeAction},
};

pub struct PeerStreamLoop {
    pub stream: TcpStream,
    pub node_action_sender: mpsc::Sender<NodeAction>,
    pub version: i32,
    pub logger_sender: mpsc::Sender<String>,
}

impl PeerStreamLoop {
    pub fn spawn(
        version: i32,
        stream: TcpStream,
        logger_sender: mpsc::Sender<String>,
        node_action_sender: mpsc::Sender<NodeAction>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut peer_action_thread = Self {
                version,
                stream,
                logger_sender,
                node_action_sender,
            };
            peer_action_thread.event_loop()
        })
    }

    pub fn event_loop(&mut self) -> Result<(), CustomError> {
        loop {
            let response_header = MessageHeader::read(&mut self.stream)?;

            match response_header.command.as_str() {
                "headers" => self.handle_headers(&response_header)?,
                "block" => self.handle_block(&response_header)?,
                "ping" => self.handle_ping(&response_header)?,
                "notfound" => self.handle_notfound(&response_header)?,
                _ => self.ignore_message(&response_header)?,
            }
        }
    }

    fn handle_headers(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let response = match Headers::read(&mut self.stream, response_header.payload_size) {
            Ok(response) => response,
            Err(_) => {
                self.node_action_sender.send(NodeAction::GetHeadersError)?;
                return Ok(());
            }
        };

        if response.headers.len() == 2000 {
            let last_header = response.headers.last().map(|h| h.hash());
            request_headers(
                last_header,
                self.version,
                &mut self.stream,
                &self.logger_sender,
                &self.node_action_sender,
            )?;
        }
        self.node_action_sender
            .send(NodeAction::NewHeaders(response))?;
        Ok(())
    }

    fn handle_block(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let block = Block::read(&mut self.stream, response_header.payload_size)?;
        match block.create_merkle_root() {
            Ok(_) => {
                self.node_action_sender
                    .send(NodeAction::Block((block.header.hash(), block)))?;
            }
            Err(_) => {
                let inventory = Inventory::new(InventoryType::GetBlock, block.header.hash());

                self.node_action_sender
                    .send(NodeAction::GetDataError(vec![inventory]))?;

                self.logger_sender.send(format!(
                    "Error validating the merkle root in the block: {:?}",
                    block.header.hash()
                ))?;
            }
        };
        Ok(())
    }

    fn handle_ping(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let ping = Ping::read(&mut self.stream, response_header.payload_size)?;
        let pong = Pong { nonce: ping.nonce };
        pong.send(&mut self.stream)?;
        Ok(())
    }

    fn handle_notfound(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let notfound = GetData::read(&mut self.stream, response_header.payload_size)?;
        let inventories = notfound.get_inventories().to_owned();
        self.node_action_sender
            .send(NodeAction::GetDataError(inventories))?;
        Ok(())
    }

    fn ignore_message(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let cmd = response_header.command.as_str();
        if cmd != "alert" && cmd != "addr" && cmd != "inv" && cmd != "sendheaders" {
            self.logger_sender.send(format!(
                "Received unknown command: {:?}",
                response_header.command
            ))?;
        }
        let mut buffer: Vec<u8> = vec![0; response_header.payload_size as usize];
        self.stream.read_exact(&mut buffer)?;
        Ok(())
    }
}