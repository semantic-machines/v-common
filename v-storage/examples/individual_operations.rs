// examples/individual_operations.rs
//! Demonstration of working with Individual objects in v-storage
//! 
//! This example shows:
//! - Storing and loading Individual objects
//! - Working with different Individual data formats (primarily JSON)
//! - Handling Individual parsing errors
//! - Different ways of working with Individual
//! 
//! Note: Individual can be represented in different formats,
//! but this example uses JSON format for clarity.

use v_storage::*;
use v_individual_model::onto::individual::Individual;

fn main() {
    println!("=== Демонстрация работы с Individual в v-storage ===\n");

    // Создаем хранилище в памяти
    let mut storage = VStorage::new(Box::new(MemoryStorage::new()));
    
    // Создаем Individual
    let mut individual = Individual::default();
    
    println!("1. Загрузка несуществующего Individual:");
    match storage.get_individual("test:nonexistent", &mut individual) {
        StorageResult::Ok(_) => {
            println!("   ✓ Individual найден!");
            println!("   • ID: {}", individual.get_id());
        },
        StorageResult::UnprocessableEntity => {
            println!("   ✗ Individual найден, но не может быть обработан");
            println!("   • Возможно, данные повреждены или имеют неверный формат");
        },
        StorageResult::NotFound => {
            println!("   ✗ Individual не найден");
            println!("   • Ключ 'test:nonexistent' не существует в хранилище");
        },
        StorageResult::NotReady => {
            println!("   ✗ Хранилище не готово");
        },
        StorageResult::Error(msg) => {
            println!("   ✗ Ошибка: {}", msg);
        }
    }
    
    println!("\n2. Сохранение Individual данных:");
    
    // Сохраняем корректные данные Individual
    let individual_data = r#"{"@":"individual","@id":"test:person","rdfs:label":[{"data":"Тестовый пользователь","lang":"ru","type":"_string"}]}"#;
    
    match storage.put_value(StorageId::Individuals, "test:person", individual_data) {
        StorageResult::Ok(_) => {
            println!("   ✓ Individual успешно сохранен");
        },
        StorageResult::Error(msg) => {
            println!("   ✗ Ошибка при сохранении: {}", msg);
        },
        _ => {
            println!("   ✗ Неожиданная ошибка при сохранении");
        }
    }
    
    println!("\n3. Загрузка существующего Individual:");
    match storage.get_individual("test:person", &mut individual) {
        StorageResult::Ok(_) => {
            println!("   ✓ Individual успешно загружен!");
            println!("   • ID: {}", individual.get_id());
            println!("   • Сырые данные: {} байт", individual.get_raw_len());
        },
        StorageResult::UnprocessableEntity => {
            println!("   ✗ Individual найден, но не может быть обработан");
            println!("   • Проверьте формат данных");
        },
        StorageResult::NotFound => {
            println!("   ✗ Individual не найден");
        },
        StorageResult::NotReady => {
            println!("   ✗ Хранилище не готово");
        },
        StorageResult::Error(msg) => {
            println!("   ✗ Ошибка: {}", msg);
        }
    }
    
    println!("\n4. Тестирование с невалидными данными:");
    
    // Сохраняем невалидные данные
    let invalid_data = "это не JSON и не Individual";
    match storage.put_value(StorageId::Individuals, "test:invalid", invalid_data) {
        StorageResult::Ok(_) => {
            println!("   ✓ Невалидные данные сохранены (для тестирования)");
        },
        StorageResult::Error(msg) => {
            println!("   ✗ Ошибка при сохранении: {}", msg);
        },
        _ => {
            println!("   ✗ Неожиданная ошибка при сохранении");
        }
    }
    
    // Пытаемся загрузить невалидные данные
    match storage.get_individual("test:invalid", &mut individual) {
        StorageResult::Ok(_) => {
            println!("   ✓ Данные загружены (неожиданно)");
        },
        StorageResult::UnprocessableEntity => {
            println!("   ✓ Корректно обработаны невалидные данные");
            println!("   • Система правильно определила, что данные не являются Individual");
        },
        StorageResult::NotFound => {
            println!("   ✗ Данные не найдены (неожиданно)");
        },
        StorageResult::NotReady => {
            println!("   ✗ Хранилище не готово");
        },
        StorageResult::Error(msg) => {
            println!("   ✗ Ошибка: {}", msg);
        }
    }
    
    println!("\n5. Работа с различными типами хранилищ:");
    
    // Демонстрация работы с разными типами хранилищ
    let mut memory_storage = VStorageEnum::memory();
    
    println!("   • Память: сохранение данных...");
    match memory_storage.put_value(StorageId::Individuals, "test:memory", individual_data) {
        StorageResult::Ok(_) => {
            println!("     ✓ Данные сохранены в памяти");
            
            // Проверяем загрузку
                         match memory_storage.get_individual(StorageId::Individuals, "test:memory", &mut individual) {
                StorageResult::Ok(_) => {
                    println!("     ✓ Данные успешно загружены из памяти");
                    println!("     • ID: {}", individual.get_id());
                },
                result => {
                    println!("     ✗ Не удалось загрузить данные: {:?}", result);
                }
            }
        },
        StorageResult::Error(msg) => {
            println!("     ✗ Ошибка при сохранении: {}", msg);
        },
        _ => {
            println!("     ✗ Неожиданная ошибка при сохранении");
        }
    }
    
    println!("\n6. Подсчет количества Individual:");
    match storage.count(StorageId::Individuals) {
        StorageResult::Ok(count) => {
            println!("   ✓ В хранилище {} Individual(s)", count);
        },
        StorageResult::Error(msg) => {
            println!("   ✗ Ошибка при подсчете: {}", msg);
        },
        _ => {
            println!("   ✗ Подсчет недоступен");
        }
    }
    
    println!("\n7. Лучшие практики:");
    println!("   • Всегда проверяйте StorageResult при загрузке Individual");
    println!("   • StorageResult::UnprocessableEntity означает, что данные есть, но не валидны");
    println!("   • StorageResult::NotFound означает, что данных нет");
    println!("   • StorageResult::NotReady означает, что хранилище не готово к работе");
    println!("   • Используйте соответствующие методы для разных типов данных");
    
    println!("\n=== Демонстрация завершена ===");
} 