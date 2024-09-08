use bevy::prelude::*;
use bevy_steamworks::{Client, LobbyId, LobbyType, SteamworksEvent, SteamworksPlugin};
use flume::{Receiver, Sender};
use steamworks::networking_types::{NetworkingIdentity, SendFlags};

#[derive(Resource)]
pub struct NetworkClient {
    lobby_id: Option<LobbyId>,
    tx: Sender<LobbyId>,
    rx: Receiver<LobbyId>
}

fn main() {
    App::new()
    .add_plugins(SteamworksPlugin::init_app(480).unwrap())
    .add_plugins(DefaultPlugins)
    .add_systems(Startup, setup)
    .add_systems(Update, (update, receive))
    .run();
}

fn setup(
    steam_client: Res<Client>,
    mut commands: Commands
) {
    println!("Connected: {}", steam_client.user().steam_id().raw());
    steam_client.networking_utils().init_relay_network_access();
    steam_client.networking_messages().session_request_callback(
        |res| {
            match res.accept() {
                true => println!("Succesfully accepted"),
                false => println!("Failed to accept"),
            }
        }
    );
    steam_client.networking_messages().session_failed_callback(
        |res| {
            println!("Session Failed: {:?}", res.end_reason().unwrap());
        }
    );
    let (tx, rx) = flume::unbounded();

    commands.insert_resource(NetworkClient {
        lobby_id: None,
        tx,
        rx,
    });
}

fn update(
    client: Res<NetworkClient>,
    steam_client: Res<Client>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        let tx: Sender<LobbyId> = client.tx.clone();
        steam_client.matchmaking().create_lobby(LobbyType::Public, 2, 
            move |res| {
                if let Ok(lobby_id) = res {
                    tx.send(lobby_id);
                }
            });
    }
    else if keys.just_pressed(KeyCode::KeyT) {
        let Some(lobby_id) = client.lobby_id else {return;};
        for player in steam_client.matchmaking().lobby_members(lobby_id) {
            if player == steam_client.user().steam_id() {
                continue;
            }
            let res = steam_client.networking_messages().send_message_to_user( NetworkingIdentity::new_steam_id(player), SendFlags::RELIABLE, &[], 0);
            match res {
                Ok(_) => println!("Message sent succesfully"),
                Err(err) => println!("Message error: {}", err.to_string()),
            }
        }
    }
}

fn receive(
    mut client: ResMut<NetworkClient>,
    steam_client: Res<Client>,
    mut evs: EventReader<SteamworksEvent>,
) {
    let rx: Receiver<LobbyId> = client.rx.clone();
    if let Ok(lobby_id) = rx.try_recv() {
        client.lobby_id = Some(lobby_id);
        println!("Joined Lobby: {}", lobby_id.raw());
    }

    let messages: Vec<steamworks::networking_types::NetworkingMessage<steamworks::ClientManager>> = steam_client.networking_messages().receive_messages_on_channel(0, 1);
    for message in messages {
        println!("Received message");
        drop(message); //not sure about usefullness, mentionned in steam docs as release
    }

    for ev in evs.read() {
        //println!("EV");
        match ev {
            SteamworksEvent::GameLobbyJoinRequested(info) => {
                println!("Trying to join: {}", info.lobby_steam_id.raw());
                let tx = client.tx.clone();
                steam_client.matchmaking().join_lobby(info.lobby_steam_id, 
                    move |res| {
                        if let Ok(lobby_id) = res {
                            match tx.send(lobby_id) {
                                Ok(_) => {}
                                Err(_) => {
                                }
                            }
                        }
                    });            },
            _ => {}
        }
    }
}