.PHONY: dev up down clean logs

dev:
	@echo "Starting Docker services..."
	docker-compose up -d
	@echo "Waiting for services to be ready..."
	@sleep 3
	@echo "Starting application..."
	cargo run

up:
	@echo "Starting Docker services..."
	docker-compose up -d

down:
	@echo "Stopping Docker services..."
	docker-compose down

clean:
	@echo "Stopping and removing Docker services..."
	docker-compose down -v

logs:
	docker-compose logs -f
