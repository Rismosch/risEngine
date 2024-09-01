use super::decl::GameObjectHandle;
use super::decl::MeshComponentHandle;
use super::id::Component;
use super::id::EcsObject;
use super::handle::GenericHandle;
use super::scene::Scene;

#[derive(Debug)]
pub struct MeshComponent {
    game_object: GameObjectHandle,
}

impl Default for MeshComponent {
    fn default() -> Self {
        let game_object = GameObjectHandle::null();
        Self{
            game_object,
        }
    }
}

impl Component for MeshComponent {
    fn create(game_object: GameObjectHandle) -> Self {
        Self {
            game_object,
        }
    }

    fn destroy(&mut self, scene: &Scene) {
    }

    fn game_object(&self) -> GameObjectHandle {
        self.game_object
    }
}
