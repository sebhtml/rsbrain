use rs_brain::{load_perceptron, train_model, Device};

fn main() {
    let device = Device::cuda().unwrap();
    let details = load_perceptron(&device).unwrap();
    train_model(details).unwrap();
}